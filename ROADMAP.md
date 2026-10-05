# Feuille de route — ForegramPM

Statut : ⬜ à faire · 🔶 en cours · ✅ fait · ❌ annulé

| # | id | Étape | Contenu | État |
|---|---|---|---|---|
| 1 | E1 | Banc de mesure | Corpus, métriques chiffrées, planche comparative ; ELK et Graphviz en références | 🔶 |
| 2 | E2 | Socle Rust | Bibliothèque sobre sans serveur ni entrées-sorties imposées, ligne de commande | ⬜ |
| 3 | E4 | Format source | Éléments identifiés, forme canonique, diagnostics ligne et JSON | ⬜ |
| 4 | E5 | MCD Merise | Germe repris, rendu maison, livré avec un premier moteur simple | ⬜ |
| 5 | E3 | Moteur de placement | Moteur en couches maison, itéré contre le banc jusqu'à battre les références | ⬜ |
| 6 | E6 | Boucle IA et placement fixable | Un agent produit des schémas réels ; position, couche, ordre, côté d'attache fixables | ⬜ |
| 7 | E10 | Feuille de route outillée | Valide les ROADMAP.md existants, applique les états, consigne d'agent, contrôle après coup | ⬜ |
| 8 | E12 | Parc | Projets déclarés, file d'attente, sortie texte et JSON, export publiable en liste blanche | ⬜ |
| 9 | E9 | Seconde notation | Une vue UML sur les mêmes éléments, vérifications croisées | ⬜ |
| 10 | E8 | Application de bureau et mobile | Aperçu des schémas, vue du parc, fil de conversation par point | ⬜ |
| 11 | E13 | Pilotage des agents | Sessions d'agent ouvertes depuis l'application, par adaptateurs | ⬜ |
| 12 | E11 | Édition graphique | Chaque geste humain réécrit le texte en forme canonique | ⬜ |

## Étape 1 — Banc de mesure  [E1]
1.1 ⬜ [P1] Format d'échange du banc : graphe abstrait en JSON (nœuds et leurs tailles, liens, côtés d'attache et contraintes en option), schéma documenté, validateur qui refuse avec un message précis. Pose l'espace de travail Rust.
1.2 ⬜ [P2] Corpus public : script rejouable qui télécharge Rome, North et elk-models et les convertit au format d'échange ; licences lues et notées, rien de versionné sans licence qui l'autorise.
1.3 ⬜ [P3] Corpus maison : MCD réels convertis avec leurs liens, plus deux cas difficiles (entité centrale à huit associations, association réflexive) ; un jeu local reste hors dépôt.
1.4 ⬜ [P4] Références : ELK et un Graphviz récent, en version figée, rendant des coordonnées dans un JSON commun ; les échecs sont listés, pas masqués.
1.5 ⬜ [P5] Métriques : croisements, coudes, longueur des liens, chevauchements, surface et proportions, écrites comme du code de produit, avec un compte rendu textuel du placement ; tests sur des cas à valeur connue.
1.6 ⬜ [P6] Rendu neutre : SVG et PNG tirés des seules coordonnées, même aspect pour tous les moteurs, pour comparer le placement et non le style.
1.7 ⬜ [P7] Planche et rapport : graphes côte à côte et tableau de métriques agrégé, régénérés par une seule commande.
1.8 ⬜ [P8] Déterminisme : même entrée, mêmes octets, sur deux exécutions puis sur deux environnements si possible ; comparaison consignée.
1.9 ⬜ [P9] Barre à battre : premier passage complet, chiffres d'ELK et de Graphviz consignés avec la version exacte de chaque moteur, comme objectif du moteur de placement.
