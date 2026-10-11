# Phase 2 — MCP et premier connecteur GitHub

Validation du 11 octobre 2026. Le dépôt contient maintenant un premier scénario contrôlé d'action externe, au-delà de la console de permissions. Cette revue a été réalisée dans le même travail que l'implémentation ; elle ne constitue pas un audit indépendant ni une certification entreprise.

## Scénario réel démontré

Un processus client distinct, utilisant le SDK MCP officiel, s'est connecté à `/mcp/` avec un credential Agent Control. Il a découvert les outils accessibles, puis demandé une création d'issue. La politique ASK a suspendu l'exécution jusqu'à votre approbation explicite du dépôt, du titre et du corps exacts, appliquée via une session humaine authentifiée.

Le connecteur a créé [l'issue #1](https://github.com/jonashvh-id/Agent-Control/issues/1) dans `jonashvh-id/Agent-Control`. Le journal relie la demande, l'approbation, le dispatch et le reçu GitHub. Après révocation du credential de l'agent, la requête suivante a reçu HTTP 401. Les preuves sont conservées dans [github-live-proof.json](github-live-proof.json) et [github-read-proof.json](github-read-proof.json).

Les appels métier réels ont été limités à une lecture d'issues et une création. Le client de démonstration est un programme MCP séparé, pas un agent piloté par un modèle de langage. Le mode local `gh-demo` a utilisé l'accès GitHub existant en mémoire, exclusivement pour ce dépôt et sur loopback ; ses droits natifs peuvent être plus larges. Aucun jeton GitHub n'a été copié dans le dépôt ou transmis au client. Un fichier privé contenant un jeton dédié est également pris en charge.

## Architecture et sécurité

La passerelle MCP, le moteur de politique, les approbations, la persistance et l'adaptateur GitHub restent distincts. Les routes REST et MCP passent par le même service de contrôle. L'admission et le dispatch ont été séparés pour libérer les transactions SQLite avant les appels réseau.

- Lecture : ALLOW uniquement avec politique et scopes actuels.
- Création : approbation obligatoire, liée aux paramètres validés exacts et à la clé d'idempotence ; aperçu humain chiffré et immuable.
- Suppression de dépôt : aucun outil ni adaptateur d'exécution enregistré ; demande refusée.
- Avant dispatch : identité, credential, scopes, agent, connecteur, arrêt global, politique, expiration et approbateur revérifiés.
- Au niveau GitHub : droits natifs toujours applicables ; refus d'authentification et de permission transmis sans exposer les secrets.
- Audit : identifiants, décisions, empreintes et reçu minimal ; pas de valeurs de requête ni de réponses brutes. La preuve réelle conserve séparément le texte de cette unique issue de validation.

Le dispatch est enregistré durablement avant l'appel externe. Une écriture incertaine n'est pas automatiquement retentée. Une révocation bloque les admissions suivantes, mais ne peut pas annuler un appel externe déjà admis.

## Résultats de vérification

| Vérification | Résultat |
|---|---|
| Tests backend et intégration | 65 réussis |
| Couverture combinée instructions et branches | 90 % ; [détails JSON](phase2-coverage.json) |
| MCP de bout en bout | Client SDK réel sur HTTP ; réponses fournisseur contrôlées, distinctes de la preuve GitHub réelle |
| Cas sensibles | Paramètres modifiés, replay, révocations, refus natif 403, accès REST, scopes, origines et absence de blocage des révocations pendant l'I/O |
| Frontend | 2 tests réussis ; ajustement limité à l'aperçu d'approbation et aux états réels |
| Ruff et dépendances | Vérifications réussies ; 78 fichiers Python formatés ; `pip check` réussi |
| Distribution | Wheel 0.2.0 installée dans un environnement séparé ; démarrage et UI autonomes réussis ; aucun secret ou runtime inclus |
| Préservation ZNode | 9 checkouts, 11 858 empreintes de fichiers et état Git inchangés |

La preuve réelle a été capturée avant l'ajout de la journalisation des refus d'authentification de credentials connus. Son HTTP 401 est enregistré dans la preuve, mais ses 12 événements d'audit ne contiennent pas cet événement supplémentaire. Le comportement ajouté est couvert par régression locale ; aucune seconde issue n'a été créée pour refaire la preuve.

La revue des trois documents demandés est dans [FOUNDATION_REVIEW.md](FOUNDATION_REVIEW.md). Le rapport d'extraction distingue désormais précisément copie, adaptation et réécriture : quatre destinations directement réutilisées représentent 169 lignes physiques, et quatre modules adaptés 106 lignes au stade phase 1. Le reste des services structurants a été réécrit pour retirer les dépendances historiques. Les empreintes de migration décrivent le socle historique, pas les fichiers modifiés en phase 2.

## Limites restantes

Authentification MCP par bearer préprovisionné, sans OAuth complet. SSO/MFA, coffre de secrets géré, isolation multi-tenant, annulation/réconciliation des opérations admises, journal externe inviolable et validation de charge/pentest restent à développer. Aucune nouvelle revue navigateur de phase 2 n'est revendiquée. Aucun déploiement public ou changement de production n'a été effectué.

La couverture mesure les chemins exécutés ; elle ne prouve pas l'absence de vulnérabilités. Le CLI reste non couvert par ce jeu de tests. Voir [SECURITY_BOUNDARIES.md](../../SECURITY_BOUNDARIES.md), [MODULE_MAP.md](../../MODULE_MAP.md) et [MCP_GITHUB.md](../api/MCP_GITHUB.md).
