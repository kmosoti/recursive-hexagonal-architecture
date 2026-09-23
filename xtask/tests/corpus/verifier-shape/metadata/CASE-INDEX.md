# Supplementary fixture index

| ID | V baseline | Mutation | Offending JSON Pointer | expected.malformed / well-formed |
| --- | --- | --- | --- | --- |
| S001 | V127 | `unchanged control` | `none` | `well formed` |
| S002 | V148 | `unchanged control` | `none` | `well formed` |
| S003 | V147 | `unchanged control` | `none` | `well formed` |
| S004 | V015 | `unchanged control` | `none` | `well formed` |
| S005 | V129 | `unchanged control` | `none` | `well formed` |
| S006 | V130 | `unchanged control` | `none` | `well formed` |
| S007 | V031 | `unchanged control` | `none` | `well formed` |
| S008 | V127 | `remove /policy` | `/policy` | `policy.digest` |
| S009 | V127 | `set /policy = []` | `/policy` | `policy.digest` |
| S010 | V127 | `remove /policy/digest` | `/policy/digest` | `policy.digest` |
| S011 | V127 | `set /policy/digest = 3` | `/policy/digest` | `policy.digest` |
| S012 | V127 | `remove /policy/profile` | `/policy/profile` | `policy.profile` |
| S013 | V127 | `set /policy/profile = 3` | `/policy/profile` | `policy.profile` |
| S014 | V127 | `remove /policy/checks` | `/policy/checks` | `policy.checks` |
| S015 | V127 | `set /policy/checks = {}` | `/policy/checks` | `policy.checks` |
| S016 | V127 | `remove /policy/rules` | `/policy/rules` | `policy.rules` |
| S017 | V127 | `set /policy/rules = {}` | `/policy/rules` | `policy.rules` |
| S018 | V127 | `remove /policy/default_obligations` | `/policy/default_obligations` | `policy.default_obligations` |
| S019 | V127 | `set /policy/default_obligations = {}` | `/policy/default_obligations` | `policy.default_obligations` |
| S020 | V127 | `set /policy/default_obligations/0 = 7` | `/policy/default_obligations/0` | `policy.default_obligations` |
| S021 | V127 | `remove /policy/non_waivable` | `/policy/non_waivable` | `policy.non_waivable` |
| S022 | V127 | `set /policy/non_waivable = {}` | `/policy/non_waivable` | `policy.non_waivable` |
| S023 | V127 | `set /policy/non_waivable/0 = 7` | `/policy/non_waivable/0` | `policy.non_waivable` |
| S024 | V127 | `remove /policy/trusted_producers` | `/policy/trusted_producers` | `policy.trusted_producers` |
| S025 | V127 | `set /policy/trusted_producers = {}` | `/policy/trusted_producers` | `policy.trusted_producers` |
| S026 | V127 | `set /policy/trusted_producers/0 = 7` | `/policy/trusted_producers/0` | `policy.trusted_producers` |
| S027 | V127 | `remove /policy/required_inputs` | `/policy/required_inputs` | `policy.required_inputs` |
| S028 | V127 | `set /policy/required_inputs = {}` | `/policy/required_inputs` | `policy.required_inputs` |
| S029 | V127 | `set /policy/required_inputs/0 = 7` | `/policy/required_inputs/0` | `policy.required_inputs` |
| S030 | V127 | `remove /policy/exception_authority` | `/policy/exception_authority` | `policy.exception_authority` |
| S031 | V127 | `set /policy/exception_authority = {}` | `/policy/exception_authority` | `policy.exception_authority` |
| S032 | V127 | `set /policy/exception_authority/0 = 7` | `/policy/exception_authority/0` | `policy.exception_authority` |
| S033 | V127 | `remove /policy/acceptance_authority` | `/policy/acceptance_authority` | `policy.acceptance_authority` |
| S034 | V127 | `set /policy/acceptance_authority = {}` | `/policy/acceptance_authority` | `policy.acceptance_authority` |
| S035 | V127 | `set /policy/acceptance_authority/0 = 7` | `/policy/acceptance_authority/0` | `policy.acceptance_authority` |
| S036 | V127 | `remove /policy/cooling_off_hours` | `/policy/cooling_off_hours` | `policy.cooling_off_hours` |
| S037 | V127 | `set /policy/cooling_off_hours = "24"` | `/policy/cooling_off_hours` | `policy.cooling_off_hours` |
| S038 | V127 | `set /policy/cooling_off_hours = -1` | `/policy/cooling_off_hours` | `policy.cooling_off_hours` |
| S039 | V127 | `set /policy/cooling_off_hours = 0.5` | `/policy/cooling_off_hours` | `policy.cooling_off_hours` |
| S040 | V127 | `set /policy/cooling_off_hours = 18446744073709551616` | `/policy/cooling_off_hours` | `policy.cooling_off_hours` |
| S041 | V127 | `set /policy/cooling_off_hours = true` | `/policy/cooling_off_hours` | `policy.cooling_off_hours` |
| S042 | V127 | `set /policy/cooling_off_hours = null` | `/policy/cooling_off_hours` | `policy.cooling_off_hours` |
| S043 | V127 | `set /policy/cooling_off_hours = 0` | `none` | `well formed` |
| S044 | V127 | `set /policy/cooling_off_hours = 18446744073709551615` | `none` | `well formed` |
| S045 | V127 | `set /policy/checks/0 = "not-an-object"` | `/policy/checks/0` | `policy.checks` |
| S046 | V127 | `remove /policy/checks/0/id` | `/policy/checks/0/id` | `policy.checks` |
| S047 | V127 | `set /policy/checks/0/id = 3` | `/policy/checks/0/id` | `policy.checks` |
| S048 | V127 | `remove /policy/checks/0/kind` | `/policy/checks/0/kind` | `policy.checks` |
| S049 | V127 | `set /policy/checks/0/kind = false` | `/policy/checks/0/kind` | `policy.checks` |
| S050 | V127 | `remove /policy/checks/0/params` | `/policy/checks/0/params` | `policy.checks` |
| S051 | V127 | `set /policy/checks/0/params = []` | `/policy/checks/0/params` | `policy.checks` |
| S052 | V127 | `set /policy/rules/0 = "not-an-object"` | `/policy/rules/0` | `policy.rules` |
| S053 | V127 | `remove /policy/rules/0/id` | `/policy/rules/0/id` | `policy.rules` |
| S054 | V127 | `set /policy/rules/0/id = 3` | `/policy/rules/0/id` | `policy.rules` |
| S055 | V127 | `remove /policy/rules/0/scope` | `/policy/rules/0/scope` | `policy.rules` |
| S056 | V127 | `set /policy/rules/0/scope = false` | `/policy/rules/0/scope` | `policy.rules` |
| S057 | V127 | `remove /policy/rules/0/requires` | `/policy/rules/0/requires` | `policy.rules` |
| S058 | V127 | `set /policy/rules/0/requires = {}` | `/policy/rules/0/requires` | `policy.rules` |
| S059 | V127 | `set /policy/rules/0/requires/0 = null` | `/policy/rules/0/requires/0` | `policy.rules` |
| S060 | V015 | `remove /local_policies` | `/local_policies` | `local_policies` |
| S061 | V015 | `set /local_policies = {}` | `/local_policies` | `local_policies` |
| S062 | V015 | `set /local_policies/0 = false` | `/local_policies/0` | `local_policies` |
| S063 | V015 | `remove /local_policies/0/scope` | `/local_policies/0/scope` | `local_policies` |
| S064 | V015 | `set /local_policies/0/scope = []` | `/local_policies/0/scope` | `local_policies` |
| S065 | V015 | `remove /local_policies/0/checks` | `/local_policies/0/checks` | `local_policies` |
| S066 | V015 | `set /local_policies/0/checks = {}` | `/local_policies/0/checks` | `local_policies` |
| S067 | V015 | `remove /local_policies/0/rules` | `/local_policies/0/rules` | `local_policies` |
| S068 | V015 | `set /local_policies/0/rules = {}` | `/local_policies/0/rules` | `local_policies` |
| S069 | V015 | `set /local_policies/0/checks/0 = "not-an-object"` | `/local_policies/0/checks/0` | `local_policies` |
| S070 | V015 | `remove /local_policies/0/checks/0/id` | `/local_policies/0/checks/0/id` | `local_policies` |
| S071 | V015 | `set /local_policies/0/checks/0/id = 3` | `/local_policies/0/checks/0/id` | `local_policies` |
| S072 | V015 | `remove /local_policies/0/checks/0/kind` | `/local_policies/0/checks/0/kind` | `local_policies` |
| S073 | V015 | `set /local_policies/0/checks/0/kind = false` | `/local_policies/0/checks/0/kind` | `local_policies` |
| S074 | V015 | `remove /local_policies/0/checks/0/params` | `/local_policies/0/checks/0/params` | `local_policies` |
| S075 | V015 | `set /local_policies/0/checks/0/params = []` | `/local_policies/0/checks/0/params` | `local_policies` |
| S076 | V015 | `set /local_policies/0/rules/0 = "not-an-object"` | `/local_policies/0/rules/0` | `local_policies` |
| S077 | V015 | `remove /local_policies/0/rules/0/id` | `/local_policies/0/rules/0/id` | `local_policies` |
| S078 | V015 | `set /local_policies/0/rules/0/id = 3` | `/local_policies/0/rules/0/id` | `local_policies` |
| S079 | V015 | `remove /local_policies/0/rules/0/scope` | `/local_policies/0/rules/0/scope` | `local_policies` |
| S080 | V015 | `set /local_policies/0/rules/0/scope = false` | `/local_policies/0/rules/0/scope` | `local_policies` |
| S081 | V015 | `remove /local_policies/0/rules/0/requires` | `/local_policies/0/rules/0/requires` | `local_policies` |
| S082 | V015 | `set /local_policies/0/rules/0/requires = {}` | `/local_policies/0/rules/0/requires` | `local_policies` |
| S083 | V015 | `set /local_policies/0/rules/0/requires/0 = null` | `/local_policies/0/rules/0/requires/0` | `local_policies` |
| S084 | V127 | `remove /base` | `/base` | `base` |
| S085 | V127 | `set /base = 12` | `/base` | `base` |
| S086 | V127 | `remove /candidate_tree` | `/candidate_tree` | `candidate_tree` |
| S087 | V127 | `set /candidate_tree = 12` | `/candidate_tree` | `candidate_tree` |
| S088 | V127 | `remove /acceptor` | `/acceptor` | `acceptor` |
| S089 | V127 | `set /acceptor = 12` | `/acceptor` | `acceptor` |
| S090 | V127 | `remove /accountable_change_authority` | `/accountable_change_authority` | `accountable_change_authority` |
| S091 | V127 | `set /accountable_change_authority = 12` | `/accountable_change_authority` | `accountable_change_authority` |
| S092 | V127 | `remove /now` | `/now` | `now` |
| S093 | V127 | `set /now = 12` | `/now` | `now` |
| S094 | V127 | `remove /surface` | `/surface` | `surface` |
| S095 | V127 | `set /surface = "crates/core/src/lib.rs"` | `/surface` | `surface` |
| S096 | V127 | `set /surface = []` | `/surface` | `surface` |
| S097 | V127 | `set /surface/0 = 7` | `/surface/0` | `surface` |
| S098 | V127 | `remove /evidence` | `/evidence` | `evidence.producer` |
| S099 | V127 | `set /evidence = []` | `/evidence` | `evidence.producer` |
| S100 | V127 | `remove /evidence/producer` | `/evidence/producer` | `evidence.producer` |
| S101 | V127 | `set /evidence/producer = 12` | `/evidence/producer` | `evidence.producer` |
| S102 | V127 | `remove /evidence/integrity` | `/evidence/integrity` | `evidence.integrity` |
| S103 | V127 | `set /evidence/integrity = 12` | `/evidence/integrity` | `evidence.integrity` |
| S104 | V127 | `remove /evidence/subject` | `/evidence/subject` | `evidence.subject` |
| S105 | V127 | `set /evidence/subject = 12` | `/evidence/subject` | `evidence.subject` |
| S106 | V127 | `remove /evidence/policy` | `/evidence/policy` | `evidence.policy` |
| S107 | V127 | `set /evidence/policy = 12` | `/evidence/policy` | `evidence.policy` |
| S108 | V127 | `remove /evidence/base` | `/evidence/base` | `evidence.base` |
| S109 | V127 | `set /evidence/base = 12` | `/evidence/base` | `evidence.base` |
| S110 | V127 | `remove /evidence/inputs` | `/evidence/inputs` | `evidence.inputs` |
| S111 | V127 | `set /evidence/inputs = []` | `/evidence/inputs` | `evidence.inputs` |
| S112 | V127 | `remove /evidence/entries` | `/evidence/entries` | `evidence.entries` |
| S113 | V127 | `set /evidence/entries = {}` | `/evidence/entries` | `evidence.entries` |
| S114 | V127 | `set /evidence/entries/0 = null` | `/evidence/entries/0` | `evidence.entries` |
| S115 | V127 | `remove /evidence/entries/0/id` | `/evidence/entries/0/id` | `evidence.entries` |
| S116 | V127 | `set /evidence/entries/0/id = 3` | `/evidence/entries/0/id` | `evidence.entries` |
| S117 | V127 | `remove /evidence/entries/0/kind` | `/evidence/entries/0/kind` | `evidence.entries` |
| S118 | V127 | `set /evidence/entries/0/kind = 3` | `/evidence/entries/0/kind` | `evidence.entries` |
| S119 | V127 | `remove /evidence/entries/0/params` | `/evidence/entries/0/params` | `evidence.entries` |
| S120 | V127 | `set /evidence/entries/0/params = []` | `/evidence/entries/0/params` | `evidence.entries` |
| S121 | V127 | `remove /evidence/entries/0/outcome` | `/evidence/entries/0/outcome` | `evidence.entries` |
| S122 | V127 | `set /evidence/entries/0/outcome = 3` | `/evidence/entries/0/outcome` | `evidence.entries` |
| S123 | V127 | `set /evidence/entries/0/selected_tests = "12"` | `/evidence/entries/0/selected_tests` | `evidence.entries` |
| S124 | V127 | `set /evidence/entries/0/selected_tests = true` | `/evidence/entries/0/selected_tests` | `evidence.entries` |
| S125 | V127 | `set /evidence/entries/0/selected_tests = null` | `/evidence/entries/0/selected_tests` | `evidence.entries` |
| S126 | V127 | `set /evidence/entries/0/selected_tests = []` | `/evidence/entries/0/selected_tests` | `evidence.entries` |
| S127 | V127 | `set /evidence/entries/0/selected_tests = {}` | `/evidence/entries/0/selected_tests` | `evidence.entries` |
| S128 | V129 | `set /evidence/entries/0/performed = 1` | `/evidence/entries/0/performed` | `evidence.entries` |
| S129 | V129 | `set /evidence/entries/0/performed = "true"` | `/evidence/entries/0/performed` | `evidence.entries` |
| S130 | V129 | `set /evidence/entries/0/performed = null` | `/evidence/entries/0/performed` | `evidence.entries` |
| S131 | V130 | `set /evidence/entries/0/corpus_digest = 1` | `/evidence/entries/0/corpus_digest` | `evidence.entries` |
| S132 | V130 | `set /evidence/entries/0/corpus_digest = []` | `/evidence/entries/0/corpus_digest` | `evidence.entries` |
| S133 | V130 | `set /evidence/entries/0/corpus_digest = null` | `/evidence/entries/0/corpus_digest` | `evidence.entries` |
| S134 | V127 | `set /evidence/entries/0/selected_tests = -1` | `none` | `well formed` |
| S135 | V127 | `set /evidence/entries/0/selected_tests = -0.5` | `none` | `well formed` |
| S136 | V127 | `set /evidence/entries/0/selected_tests = 0.5` | `none` | `well formed` |
| S137 | V127 | `set /evidence/entries/0/selected_tests = 1.5` | `none` | `well formed` |
| S138 | V127 | `set /evidence/entries/0/selected_tests = 0` | `none` | `well formed` |
| S139 | V127 | `set /evidence/entries/0/selected_tests = 18446744073709551616` | `none` | `well formed` |
| S140 | V127 | `set /evidence/entries/0/selected_tests = 1` | `none` | `well formed` |
| S141 | V127 | `set /evidence/entries/0/selected_tests = 12` | `none` | `well formed` |
| S142 | V127 | `set /evidence/entries/0/selected_tests = 18446744073709551615` | `none` | `well formed` |
| S143 | V127 | `remove /evidence/entries/0/selected_tests` | `none` | `well formed` |
| S144 | V129 | `remove /evidence/entries/0/performed` | `none` | `well formed` |
| S145 | V130 | `remove /evidence/entries/0/corpus_digest` | `none` | `well formed` |
| S146 | V127 | `set /evidence/entries/0/outcome = "failed"` | `none` | `well formed` |
| S147 | V127 | `set /evidence/entries/0/outcome = "not_run"` | `none` | `well formed` |
| S148 | V127 | `set /evidence/entries/0/outcome = "not_applicable"` | `none` | `well formed` |
| S149 | V127 | `set /evidence/entries/0/outcome = "inconclusive"` | `none` | `well formed` |
| S150 | V127 | `set /evidence/entries/0/outcome = "unknown-outcome"` | `none` | `well formed` |
| S151 | V127 | `remove /exception` | `/exception` | `exception` |
| S152 | V127 | `set /exception = false` | `/exception` | `exception` |
| S153 | V127 | `set /exception = "none"` | `/exception` | `exception` |
| S154 | V127 | `set /exception = []` | `/exception` | `exception` |
| S155 | V148 | `remove /exception/subject` | `/exception/subject` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S156 | V148 | `set /exception/subject = 7` | `/exception/subject` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S157 | V148 | `remove /exception/base` | `/exception/base` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S158 | V148 | `set /exception/base = 7` | `/exception/base` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S159 | V148 | `remove /exception/policy` | `/exception/policy` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S160 | V148 | `set /exception/policy = 7` | `/exception/policy` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S161 | V148 | `remove /exception/issuer` | `/exception/issuer` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S162 | V148 | `set /exception/issuer = 7` | `/exception/issuer` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S163 | V148 | `remove /exception/issued_at` | `/exception/issued_at` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S164 | V148 | `set /exception/issued_at = 7` | `/exception/issued_at` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S165 | V148 | `remove /exception/expires_at` | `/exception/expires_at` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S166 | V148 | `set /exception/expires_at = 7` | `/exception/expires_at` | `exception.subject, base, policy, issuer, issued_at, expires_at` |
| S167 | V148 | `remove /exception/reason` | `/exception/reason` | `exception.reason, compensating_control, follow_up` |
| S168 | V148 | `set /exception/reason = 7` | `/exception/reason` | `exception.reason, compensating_control, follow_up` |
| S169 | V148 | `remove /exception/compensating_control` | `/exception/compensating_control` | `exception.reason, compensating_control, follow_up` |
| S170 | V148 | `set /exception/compensating_control = 7` | `/exception/compensating_control` | `exception.reason, compensating_control, follow_up` |
| S171 | V148 | `remove /exception/follow_up` | `/exception/follow_up` | `exception.reason, compensating_control, follow_up` |
| S172 | V148 | `set /exception/follow_up = 7` | `/exception/follow_up` | `exception.reason, compensating_control, follow_up` |
| S173 | V148 | `remove /exception/authentic` | `/exception/authentic` | `exception.authentic, revoked` |
| S174 | V148 | `set /exception/authentic = "true"` | `/exception/authentic` | `exception.authentic, revoked` |
| S175 | V148 | `remove /exception/revoked` | `/exception/revoked` | `exception.authentic, revoked` |
| S176 | V148 | `set /exception/revoked = "true"` | `/exception/revoked` | `exception.authentic, revoked` |
| S177 | V148 | `remove /exception/waived` | `/exception/waived` | `exception.waived` |
| S178 | V148 | `set /exception/waived = {}` | `/exception/waived` | `exception.waived` |
| S179 | V148 | `set /exception/waived/0 = 7` | `/exception/waived/0` | `exception.waived` |
| S180 | V148 | `set /exception/reason = ""` | `none` | `well formed` |
| S181 | V148 | `set /exception/compensating_control = ""` | `none` | `well formed` |
| S182 | V148 | `set /exception/follow_up = ""` | `none` | `well formed` |
| S183 | V147 | `remove /exception/logged_at` | `none` | `well formed` |
| S184 | V147 | `set /exception/logged_at = "not-a-timestamp"` | `none` | `well formed` |
| S185 | V147 | `set /exception/logged_at = 7` | `none` | `well formed` |
| S186 | V147 | `set /exception/logged_at = null` | `none` | `well formed` |
| S187 | V147 | `set /exception/logged_at = {}` | `none` | `well formed` |
| S188 | V148 | `set /exception/logged_at = {"unused": true}` | `none` | `well formed` |
