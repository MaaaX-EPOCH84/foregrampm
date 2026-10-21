// ForegramPM — measurement bench: ELK driver. Reads graphs on standard input,
// places them with ELK Layered, renders absolute coordinates.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later
//
// A single process places all the graphs of a run: the virtual machine
// starts only once. The dialogue is line by line, fields separated by
// a tab (forbidden in an identifier), `-` for an absent value,
// lengths in pixels. The bench (`engine/elk.rs`) writes:
//
//   graph  NAME DIRECTION
//   node   ID PARENT WIDTH HEIGHT PADDING CONSTRAINTS   (parent before child)
//   port   ID NODE SIDE X Y WIDTH HEIGHT INDEX          (X, Y: anchor point)
//   edge   ID SOURCE TARGET SOURCE_PORT TARGET_PORT SOURCE_SIDE TARGET_SIDE
//   label  EDGE WIDTH HEIGHT AT
//   layout
//
// and receives, everything in absolute terms:
//
//   layout NAME
//   node   ID X Y WIDTH HEIGHT
//   port   NODE ID X Y WIDTH HEIGHT                     (corner of the box)
//   edge   ID X Y [X Y...]                              (start, bends, end)
//   label  EDGE X Y
//   end
//
// or `fail NAME MESSAGE` then `end` if ELK failed or timed out.

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
    /// Time allowed for one layout, in seconds; beyond it the graph fails.
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

    /// A separate worker thread, so that a layout that runs too long can be abandoned;
    /// daemon, so that the virtual machine exits even if it is still running.
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

    /// Builds ELK's graph from the lines received, then lays it out.
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

        /// The format anchors a port on the border, box centred on the anchor; ELK
        /// places a port by its corner and puts the box outside the border, at
        /// `borderOffset`: half the box towards the inside recentres it.
        /// With no known side, it is half the width; a box that is not
        /// square may then end up off the border, which the validator reports.
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

        /// A forced side without a port becomes an unnamed port, fixed on that side;
        /// it is not rendered.
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

        /// ELK ignores a label without text: the text is a dummy, only the size
        /// matters.
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
            // Without this option, ELK lays out each level separately and does not route
            // links that cross a container.
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
