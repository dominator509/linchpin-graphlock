# Anti-Gaming Control Baseline

The generic scanner intentionally reports words such as placeholder, simulate, mock and bypass wherever they occur. In this blueprint, many expected hits are the control law, scanner pattern definitions, DOD/test registries, or security test corpus describing forbidden behavior. Those are not product-code violations.

No production product source exists in FORGE mode. Therefore this baseline does not whitelist future source paths and does not convert scanner matches to PASS. During execution each match touching product code, tests, configuration or evidence must be classified with file/line/rationale. Unclassified matches block closure; any fake production-path behavior, gate bypass, hardcoded pass, unproven fallback, or test-only implementation on a release path fails the node.
