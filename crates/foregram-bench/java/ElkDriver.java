// ForegramPM — banc de mesure : pilote d'ELK. Lit des graphes sur l'entrée
// standard, les place avec ELK Layered, rend les coordonnées absolues.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Un seul processus place tous les graphes d'un passage : la machine virtuelle
// ne démarre qu'une fois. Le dialogue est ligne à ligne, champs séparés par
// une tabulation (interdite dans un identifiant), `-` pour une valeur absente,
// longueurs en pixels. Le banc (`engine/elk.rs`) écrit :
//
//   graph  NAME DIRECTION
//   node   ID PARENT WIDTH HEIGHT PADDING CONSTRAINTS   (parent avant enfant)
//   port   ID NODE SIDE X Y WIDTH HEIGHT INDEX          (X, Y : point d'ancrage)
//   edge   ID SOURCE TARGET SOURCE_PORT TARGET_PORT SOURCE_SIDE TARGET_SIDE
//   label  EDGE WIDTH HEIGHT AT
//   layout
//
// et reçoit, tout en absolu :
//
//   layout NAME
//   node   ID X Y WIDTH HEIGHT
//   port   NODE ID X Y WIDTH HEIGHT                     (coin de la boîte)
//   edge   ID X Y [X Y...]                              (départ, coudes, arrivée)
//   label  EDGE X Y
//   end
//
// ou `fail NAME MESSAGE` puis `end` si ELK a échoué ou dépassé le délai.

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

import org.eclipse.elk.alg.layered.options.LayeredMetaDataProvider;
import org.eclipse.elk.core.RecursiveGraphLayoutEngine;
import org.eclipse.elk.core.data.LayoutMetaDataService;
import org.eclipse.elk.core.math.ElkPadding;
import org.eclipse.elk.core.math.KVector;
import org.eclipse.elk.core.options.CoreOptions;
import org.eclipse.elk.core.options.Direction;
import org.eclipse.elk.core.options.EdgeLabelPlacement;
import org.eclipse.elk.core.options.HierarchyHandling;
import org.eclipse.elk.core.options.PortConstraints;
import org.eclipse.elk.core.options.PortSide;
import org.eclipse.elk.core.util.BasicProgressMonitor;
import org.eclipse.elk.core.util.ElkUtil;
import org.eclipse.elk.graph.ElkBendPoint;
import org.eclipse.elk.graph.ElkConnectableShape;
import org.eclipse.elk.graph.ElkEdge;
import org.eclipse.elk.graph.ElkEdgeSection;
import org.eclipse.elk.graph.ElkLabel;
import org.eclipse.elk.graph.ElkNode;
import org.eclipse.elk.graph.ElkPort;
import org.eclipse.elk.graph.util.ElkGraphUtil;

public final class ElkDriver {
    /// Délai accordé à un placement, en secondes ; au-delà le graphe échoue.
    private static final long TIMEOUT_SECONDS = 60;

    private ElkDriver() {
    }

    public static void main(String[] args) throws Exception {
        LayoutMetaDataService.getInstance().registerLayoutMetaDataProviders(new LayeredMetaDataProvider());
        BufferedReader in = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
        PrintStream out = new PrintStream(System.out, false, StandardCharsets.UTF_8);
        ExecutorService worker = newWorker();
        Builder builder = null;
        String line;
        while ((line = in.readLine()) != null) {
            String[] f = line.split("\t", -1);
            if (f[0].equals("graph")) {
                builder = new Builder(f[1], f[2]);
                continue;
            }
            if (builder == null) {
                continue;
            }
            if (f[0].equals("layout")) {
                Builder done = builder;
                builder = null;
                Future<String> reply = worker.submit(() -> done.layout());
                try {
                    out.print(reply.get(TIMEOUT_SECONDS, TimeUnit.SECONDS));
                } catch (TimeoutException e) {
                    reply.cancel(true);
                    worker.shutdownNow();
                    worker = newWorker();
                    out.print("fail\t" + done.name + "\ttimeout\nend\n");
                } catch (Exception e) {
                    out.print("fail\t" + done.name + "\t" + oneLine(e) + "\nend\n");
                }
                out.flush();
                continue;
            }
            try {
                builder.add(f);
            } catch (RuntimeException e) {
                builder.broken = oneLine(e);
            }
        }
        worker.shutdownNow();
        out.flush();
    }

    /// Un fil de travail à part, pour pouvoir abandonner un placement trop long ;
    /// démon, pour que la machine virtuelle s'arrête même s'il tourne encore.
    private static ExecutorService newWorker() {
        return Executors.newSingleThreadExecutor(r -> {
            Thread t = new Thread(r, "layout");
            t.setDaemon(true);
            return t;
        });
    }

    private static String oneLine(Throwable e) {
        Throwable cause = e;
        while (cause.getCause() != null && cause != cause.getCause()) {
            cause = cause.getCause();
        }
        String text = cause.getClass().getSimpleName() + (cause.getMessage() == null ? "" : ": " + cause.getMessage());
        return text.replaceAll("[\\t\\r\\n]+", " ");
    }

    /// Construit le graphe d'ELK à partir des lignes reçues, puis le place.
    private static final class Builder {
        final String name;
        final ElkNode root = ElkGraphUtil.createGraph();
        final Map<String, ElkNode> nodes = new HashMap<>();
        final Map<String, ElkPort> ports = new HashMap<>();
        final Map<String, ElkEdge> edges = new HashMap<>();
        final List<ElkEdge> edgeOrder = new ArrayList<>();
        final List<ElkLabel> labels = new ArrayList<>();
        final List<String> labelEdges = new ArrayList<>();
        boolean nested = false;
        String broken = null;

        Builder(String name, String direction) {
            this.name = name;
            root.setProperty(CoreOptions.ALGORITHM, "org.eclipse.elk.layered");
            if (!direction.equals("-")) {
                root.setProperty(CoreOptions.DIRECTION, Direction.valueOf(direction.toUpperCase()));
            }
        }

        void add(String[] f) {
            switch (f[0]) {
            case "node":
                node(f);
                break;
            case "port":
                port(f);
                break;
            case "edge":
                edge(f);
                break;
            case "label":
                label(f);
                break;
            default:
                throw new IllegalArgumentException("unknown line " + f[0]);
            }
        }

        private void node(String[] f) {
            ElkNode parent = f[2].equals("-") ? root : need(nodes, f[2], "node");
            if (parent != root) {
                nested = true;
            }
            ElkNode node = ElkGraphUtil.createNode(parent);
            node.setIdentifier(f[1]);
            if (!f[3].equals("-")) {
                node.setDimensions(px(f[3]), px(f[4]));
            }
            if (!f[5].equals("-")) {
                node.setProperty(CoreOptions.PADDING, new ElkPadding(px(f[5])));
            }
            node.setProperty(CoreOptions.PORT_CONSTRAINTS, PortConstraints.valueOf(f[6]));
            nodes.put(f[1], node);
        }

        /// Le format ancre un port sur le bord, boîte centrée sur l'ancre ; ELK
        /// place un port par son coin et pose la boîte en dehors du bord, à
        /// `borderOffset` : la moitié de la boîte vers l'intérieur la recentre.
        /// Sans côté connu, c'est la moitié de la largeur ; une boîte qui n'est
        /// pas carrée peut alors finir hors du bord, ce que le validateur relève.
        private void port(String[] f) {
            ElkNode node = need(nodes, f[2], "node");
            ElkPort port = ElkGraphUtil.createPort(node);
            port.setIdentifier(f[1]);
            double w = f[6].equals("-") ? 0 : px(f[6]);
            double h = f[7].equals("-") ? 0 : px(f[7]);
            port.setDimensions(w, h);
            PortSide side = f[3].equals("-") ? null : PortSide.valueOf(f[3].toUpperCase());
            boolean vertical = side == PortSide.NORTH || side == PortSide.SOUTH;
            port.setProperty(CoreOptions.PORT_BORDER_OFFSET, -(vertical ? h : w) / 2);
            if (side != null) {
                port.setProperty(CoreOptions.PORT_SIDE, side);
            }
            if (!f[4].equals("-")) {
                port.setLocation(px(f[4]) - w / 2, px(f[5]) - h / 2);
            }
            if (!f[8].equals("-")) {
                port.setProperty(CoreOptions.PORT_INDEX, Integer.parseInt(f[8]));
            }
            ports.put(f[1], port);
        }

        /// Un côté imposé sans port devient un port sans nom, fixé sur ce côté ;
        /// il n'est pas rendu.
        private void edge(String[] f) {
            ElkConnectableShape source = end(f[2], f[4], f[6]);
            ElkConnectableShape target = end(f[3], f[5], f[7]);
            ElkEdge edge = ElkGraphUtil.createSimpleEdge(source, target);
            edge.setIdentifier(f[1]);
            edges.put(f[1], edge);
            edgeOrder.add(edge);
        }

        private ElkConnectableShape end(String nodeId, String portId, String side) {
            if (!portId.equals("-")) {
                return need(ports, portId, "port");
            }
            ElkNode node = need(nodes, nodeId, "node");
            if (side.equals("-")) {
                return node;
            }
            ElkPort port = ElkGraphUtil.createPort(node);
            port.setDimensions(0, 0);
            port.setProperty(CoreOptions.PORT_SIDE, PortSide.valueOf(side.toUpperCase()));
            port.setProperty(CoreOptions.PORT_BORDER_OFFSET, 0.0);
            return port;
        }

        /// ELK ignore une étiquette sans texte : le texte est factice, la taille
        /// seule compte.
        private void label(String[] f) {
            ElkEdge edge = need(edges, f[1], "edge");
            ElkLabel label = ElkGraphUtil.createLabel(edge);
            label.setText("l");
            label.setDimensions(px(f[2]), px(f[3]));
            EdgeLabelPlacement at = f[4].equals("source") ? EdgeLabelPlacement.TAIL
                    : f[4].equals("target") ? EdgeLabelPlacement.HEAD : EdgeLabelPlacement.CENTER;
            label.setProperty(CoreOptions.EDGE_LABELS_PLACEMENT, at);
            labels.add(label);
            labelEdges.add(f[1]);
        }

        private static <T> T need(Map<String, T> map, String id, String kind) {
            T found = map.get(id);
            if (found == null) {
                throw new IllegalArgumentException("unknown " + kind + " " + id);
            }
            return found;
        }

        private static double px(String text) {
            return Double.parseDouble(text);
        }

        String layout() {
            if (broken != null) {
                return "fail\t" + name + "\t" + broken + "\nend\n";
            }
            // Sans cette option, ELK place chaque niveau à part et ne trace pas
            // les liens qui traversent un conteneur.
            if (nested) {
                root.setProperty(CoreOptions.HIERARCHY_HANDLING, HierarchyHandling.INCLUDE_CHILDREN);
            }
            new RecursiveGraphLayoutEngine().layout(root, new BasicProgressMonitor());
            StringBuilder out = new StringBuilder();
            out.append("layout\t").append(name).append('\n');
            for (ElkNode node : root.getChildren()) {
                dump(node, out);
            }
            for (ElkEdge edge : edgeOrder) {
                KVector origin = new KVector();
                ElkUtil.toAbsolute(origin, edge.getContainingNode());
                out.append("edge\t").append(edge.getIdentifier());
                for (ElkEdgeSection section : edge.getSections()) {
                    point(out, section.getStartX(), section.getStartY(), origin);
                    for (ElkBendPoint bend : section.getBendPoints()) {
                        point(out, bend.getX(), bend.getY(), origin);
                    }
                    point(out, section.getEndX(), section.getEndY(), origin);
                }
                out.append('\n');
            }
            for (int i = 0; i < labels.size(); i++) {
                ElkLabel label = labels.get(i);
                KVector origin = new KVector();
                ElkUtil.toAbsolute(origin, ((ElkEdge) label.getParent()).getContainingNode());
                out.append("label\t").append(labelEdges.get(i));
                point(out, label.getX(), label.getY(), origin);
                out.append('\n');
            }
            out.append("end\n");
            return out.toString();
        }

        private static void dump(ElkNode node, StringBuilder out) {
            KVector abs = new KVector();
            ElkUtil.toAbsolute(abs, node);
            out.append("node\t").append(node.getIdentifier()).append('\t').append(abs.x).append('\t').append(abs.y)
                    .append('\t').append(node.getWidth()).append('\t').append(node.getHeight()).append('\n');
            for (ElkPort port : node.getPorts()) {
                if (port.getIdentifier() == null) {
                    continue;
                }
                out.append("port\t").append(node.getIdentifier()).append('\t').append(port.getIdentifier())
                        .append('\t').append(abs.x + port.getX()).append('\t').append(abs.y + port.getY())
                        .append('\t').append(port.getWidth()).append('\t').append(port.getHeight()).append('\n');
            }
            for (ElkNode child : node.getChildren()) {
                dump(child, out);
            }
        }

        private static void point(StringBuilder out, double x, double y, KVector origin) {
            out.append('\t').append(x + origin.x).append('\t').append(y + origin.y);
        }
    }
}
