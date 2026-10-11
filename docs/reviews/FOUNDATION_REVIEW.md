# Revue du socle avant phase 2

Cette revue examine le code et les trois documents demandés : EXTRACTION_REPORT, MODULE_MAP et SECURITY_BOUNDARIES. Elle est effectuée dans le même chat que l'implémentation ; ce n'est pas un audit indépendant.

La modularité est présente. Avant phase 2, le gateway faisait 232 lignes, le moteur de politique 23 lignes et le dépôt des approbations 89 lignes. Les routes, services, repositories et adaptateurs avaient des responsabilités séparées. Aucune dépendance d'exécution à ZNode n'a été trouvée.

La réutilisation doit être décrite avec précision : deux fichiers autonomes copiés intégralement, deux extractions de primitives, quatre modules adaptés. Les autres services structurants ont été réécrits. Les quatre fichiers de réutilisation directe représentent 169 lignes physiques, 136 hors lignes vides/commentaires. Ce résultat répond à l'indépendance, mais ne permet pas de présenter tout le backend comme du code ZNode éprouvé.

Les 33 tests passent. La couverture combinant instructions et branches est 88 % sur le backend. Le gateway et le repository des approbations sont à 96 %, le moteur de politique à 80 %. Le CLI n'est pas couvert par ce jeu de tests, et certains chemins de configuration, stockage, authentification et erreurs le sont partiellement. Le JSON associé donne les lignes et branches manquantes.

## Constats à traiter pour le scénario externe

1. **Approbation peu informative.** Seuls le nombre et les types de paramètres étaient visibles. Il faut montrer le dépôt, le titre et le corps exacts pour que la décision humaine soit utile. En phase 2, cet aperçu est chiffré, immuable et réservé aux sessions humaines autorisées ; les valeurs restent absentes de l'audit.
2. **Transaction ouverte pendant l'appel externe.** Le gateway maintenait un verrou d'écriture SQLite durant le dispatch. En phase 2, l'admission définitive est enregistrée avant l'I/O, puis le résultat dans une transaction séparée. Les révocations ne sont plus bloquées par le réseau. Une opération déjà admise peut néanmoins finir.
3. **Aucun transport MCP ni adaptateur réel.** Les tests de la fondation utilisaient un adaptateur de fixture. En phase 2, un client SDK officiel et le connecteur REST GitHub sont testés ensemble ; un test réel distinct doit être identifié comme tel.
4. **Réutilisation et tests surestimables.** Les tests ZNode dépendants du harness historique ont été remplacés par des scénarios autonomes. Ils n'ont pas été migrés intégralement ni exécutés contre ZNode. Le rapport d'extraction distingue maintenant copie, adaptation et réécriture.
5. **Limites persistantes.** Pas d'OAuth MCP complet, SSO/MFA, coffre de secrets géré, isolation multi-tenant, preuve externe d'intégrité du journal, annulation des opérations admises ou certification entreprise. Le mode gh-demo réutilise un accès local potentiellement large et est réservé au dépôt de test/loopback.

ZNode est resté inchangé : neuf checkouts, HEADs/refs/statuts Git et 11 858 empreintes de fichiers comparés au relevé initial.
