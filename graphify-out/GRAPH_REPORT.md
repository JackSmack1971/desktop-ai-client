# Graph Report - .  (2026-06-20)

## Corpus Check
- 204 files · ~176,264 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1424 nodes · 2580 edges · 119 communities (99 shown, 20 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 98 edges (avg confidence: 0.81)
- Token cost: 1,485,721 input · 43,389 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Community 0|Community 0]]
- [[_COMMUNITY_Community 1|Community 1]]
- [[_COMMUNITY_Community 2|Community 2]]
- [[_COMMUNITY_Community 3|Community 3]]
- [[_COMMUNITY_Community 4|Community 4]]
- [[_COMMUNITY_Community 5|Community 5]]
- [[_COMMUNITY_Community 6|Community 6]]
- [[_COMMUNITY_Community 7|Community 7]]
- [[_COMMUNITY_Community 8|Community 8]]
- [[_COMMUNITY_Community 9|Community 9]]
- [[_COMMUNITY_Community 10|Community 10]]
- [[_COMMUNITY_Community 11|Community 11]]
- [[_COMMUNITY_Community 12|Community 12]]
- [[_COMMUNITY_Community 13|Community 13]]
- [[_COMMUNITY_Community 14|Community 14]]
- [[_COMMUNITY_Community 15|Community 15]]
- [[_COMMUNITY_Community 16|Community 16]]
- [[_COMMUNITY_Community 17|Community 17]]
- [[_COMMUNITY_Community 18|Community 18]]
- [[_COMMUNITY_Community 19|Community 19]]
- [[_COMMUNITY_Community 20|Community 20]]
- [[_COMMUNITY_Community 21|Community 21]]
- [[_COMMUNITY_Community 22|Community 22]]
- [[_COMMUNITY_Community 23|Community 23]]
- [[_COMMUNITY_Community 24|Community 24]]
- [[_COMMUNITY_Community 25|Community 25]]
- [[_COMMUNITY_Community 26|Community 26]]
- [[_COMMUNITY_Community 27|Community 27]]
- [[_COMMUNITY_Community 28|Community 28]]
- [[_COMMUNITY_Community 29|Community 29]]
- [[_COMMUNITY_Community 30|Community 30]]
- [[_COMMUNITY_Community 31|Community 31]]
- [[_COMMUNITY_Community 32|Community 32]]
- [[_COMMUNITY_Community 33|Community 33]]
- [[_COMMUNITY_Community 34|Community 34]]
- [[_COMMUNITY_Community 35|Community 35]]
- [[_COMMUNITY_Community 36|Community 36]]
- [[_COMMUNITY_Community 37|Community 37]]
- [[_COMMUNITY_Community 38|Community 38]]
- [[_COMMUNITY_Community 39|Community 39]]
- [[_COMMUNITY_Community 40|Community 40]]
- [[_COMMUNITY_Community 41|Community 41]]
- [[_COMMUNITY_Community 42|Community 42]]
- [[_COMMUNITY_Community 43|Community 43]]
- [[_COMMUNITY_Community 44|Community 44]]
- [[_COMMUNITY_Community 45|Community 45]]
- [[_COMMUNITY_Community 46|Community 46]]
- [[_COMMUNITY_Community 47|Community 47]]
- [[_COMMUNITY_Community 48|Community 48]]
- [[_COMMUNITY_Community 49|Community 49]]
- [[_COMMUNITY_Community 50|Community 50]]
- [[_COMMUNITY_Community 51|Community 51]]
- [[_COMMUNITY_Community 52|Community 52]]
- [[_COMMUNITY_Community 53|Community 53]]
- [[_COMMUNITY_Community 54|Community 54]]
- [[_COMMUNITY_Community 55|Community 55]]
- [[_COMMUNITY_Community 56|Community 56]]
- [[_COMMUNITY_Community 57|Community 57]]
- [[_COMMUNITY_Community 58|Community 58]]
- [[_COMMUNITY_Community 59|Community 59]]
- [[_COMMUNITY_Community 60|Community 60]]
- [[_COMMUNITY_Community 61|Community 61]]
- [[_COMMUNITY_Community 62|Community 62]]
- [[_COMMUNITY_Community 63|Community 63]]
- [[_COMMUNITY_Community 64|Community 64]]
- [[_COMMUNITY_Community 65|Community 65]]
- [[_COMMUNITY_Community 66|Community 66]]
- [[_COMMUNITY_Community 67|Community 67]]
- [[_COMMUNITY_Community 68|Community 68]]
- [[_COMMUNITY_Community 69|Community 69]]
- [[_COMMUNITY_Community 70|Community 70]]
- [[_COMMUNITY_Community 71|Community 71]]
- [[_COMMUNITY_Community 72|Community 72]]
- [[_COMMUNITY_Community 73|Community 73]]
- [[_COMMUNITY_Community 74|Community 74]]
- [[_COMMUNITY_Community 75|Community 75]]
- [[_COMMUNITY_Community 76|Community 76]]
- [[_COMMUNITY_Community 77|Community 77]]
- [[_COMMUNITY_Community 78|Community 78]]
- [[_COMMUNITY_Community 79|Community 79]]
- [[_COMMUNITY_Community 80|Community 80]]
- [[_COMMUNITY_Community 81|Community 81]]
- [[_COMMUNITY_Community 82|Community 82]]
- [[_COMMUNITY_Community 83|Community 83]]
- [[_COMMUNITY_Community 84|Community 84]]
- [[_COMMUNITY_Community 85|Community 85]]
- [[_COMMUNITY_Community 86|Community 86]]
- [[_COMMUNITY_Community 87|Community 87]]
- [[_COMMUNITY_Community 88|Community 88]]
- [[_COMMUNITY_Community 93|Community 93]]
- [[_COMMUNITY_Community 105|Community 105]]
- [[_COMMUNITY_Community 108|Community 108]]
- [[_COMMUNITY_Community 110|Community 110]]
- [[_COMMUNITY_Community 112|Community 112]]
- [[_COMMUNITY_Community 113|Community 113]]
- [[_COMMUNITY_Community 114|Community 114]]
- [[_COMMUNITY_Community 115|Community 115]]
- [[_COMMUNITY_Community 116|Community 116]]

## God Nodes (most connected - your core abstractions)
1. `resolve_execution_profile()` - 32 edges
2. `run_migrations()` - 28 edges
3. `seed_trace()` - 21 edges
4. `Prompt Blueprint` - 21 edges
5. `chat_send()` - 19 edges
6. `migrated_pool()` - 19 edges
7. `AppState` - 18 edges
8. `String` - 18 edges
9. `drive_byte_stream()` - 18 edges
10. `resolve_attachments()` - 17 edges

## Surprising Connections (you probably didn't know these)
- `Add guardrails at decision points` --semantically_similar_to--> `resolve_execution_profile()`  [INFERRED] [semantically similar]
  docs/prompt-blueprint.md → src-tauri/src/providers/policy.rs
- `resolve_execution_profile()` --references--> `PolicyError::PrivacyUnsatisfied`  [EXTRACTED]
  src-tauri/src/providers/policy.rs → docs/provider-routing.md
- `src-tauri/tests/app_shell.rs` --shares_data_with--> `migrated_pool()`  [EXTRACTED]
  plans/011-unify-shell-lock-order.md → src-tauri/tests/app_shell.rs
- `get_active_surface()` --calls--> `Sqlite store with_conn / Mutex<Connection>`  [EXTRACTED]
  src-tauri/src/ipc/app_shell.rs → plans/011-unify-shell-lock-order.md
- `set_active_surface()` --calls--> `Sqlite store with_conn / Mutex<Connection>`  [EXTRACTED]
  src-tauri/src/ipc/app_shell.rs → plans/011-unify-shell-lock-order.md

## Import Cycles
- 1-file cycle: `src-tauri/build.rs -> src-tauri/build.rs`
- 1-file cycle: `src-tauri/src/app_state.rs -> src-tauri/src/app_state.rs`
- 1-file cycle: `src-tauri/src/ipc/app_shell.rs -> src-tauri/src/ipc/app_shell.rs`
- 1-file cycle: `src-tauri/src/ipc/chat.rs -> src-tauri/src/ipc/chat.rs`
- 1-file cycle: `src-tauri/src/ipc/history.rs -> src-tauri/src/ipc/history.rs`
- 1-file cycle: `src-tauri/src/ipc/files.rs -> src-tauri/src/ipc/files.rs`
- 1-file cycle: `src-tauri/src/storage/memory.rs -> src-tauri/src/storage/memory.rs`
- 1-file cycle: `src-tauri/src/ipc/inventory.rs -> src-tauri/src/ipc/inventory.rs`
- 1-file cycle: `src-tauri/src/ipc/privacy.rs -> src-tauri/src/ipc/privacy.rs`
- 1-file cycle: `src-tauri/src/ipc_inventory_prop_tests.rs -> src-tauri/src/ipc_inventory_prop_tests.rs`
- 1-file cycle: `src-tauri/src/providers/routing.rs -> src-tauri/src/providers/routing.rs`
- 1-file cycle: `src-tauri/src/security/artifact_sandbox.rs -> src-tauri/src/security/artifact_sandbox.rs`
- 1-file cycle: `src-tauri/src/security/attachment_budget.rs -> src-tauri/src/security/attachment_budget.rs`
- 1-file cycle: `src-tauri/src/storage/artifacts.rs -> src-tauri/src/storage/artifacts.rs`
- 1-file cycle: `src-tauri/src/storage/fts.rs -> src-tauri/src/storage/fts.rs`
- 1-file cycle: `src-tauri/src/storage/retention.rs -> src-tauri/src/storage/retention.rs`
- 1-file cycle: `src-tauri/src/storage/sqlite.rs -> src-tauri/src/storage/sqlite.rs`
- 1-file cycle: `src-tauri/src/storage/turns.rs -> src-tauri/src/storage/turns.rs`
- 1-file cycle: `src-tauri/src/telemetry/memory_replay.rs -> src-tauri/src/telemetry/memory_replay.rs`
- 1-file cycle: `src-tauri/src/telemetry/release_evidence.rs -> src-tauri/src/telemetry/release_evidence.rs`

## Hyperedges (group relationships)
- **Shared SCREAMING_SNAKE_CASE IPC error shape pattern** — chat_chaterror, history_historyerror, files_fileserror, privacy_privacyerror, artifacts_artifacterror, app_shell_shellerror [INFERRED 0.85]
- **All IPC commands enforcing main-window-only policy_check** — chat_chat_send, chat_chat_cancel, history_history_list, history_history_get, history_history_delete, history_history_search, files_files_open_dialog, files_files_read_token, privacy_privacy_set_provider_key, privacy_privacy_get_credential_status, privacy_privacy_clear_provider_key, artifacts_artifact_get, artifacts_artifact_dismiss, app_shell_get_active_surface, app_shell_set_active_surface [EXTRACTED 1.00]
- **Command inventory cross-check chain (inventory toml, main.rs handler, capability file, permission files, compiled allowlist)** — inventory_command_inventory_toml, inventory_release_capabilities_toml, main_main_json, inventory_verify_inventory, inventory_registered_commands_from_main_rs, inventory_compiled_command_allowlist [EXTRACTED 1.00]
- **Conversation Transaction Protocol participants** — turns_turnstore, turns_begin_turn, turns_start_attempt, turns_complete_attempt_success, turns_complete_attempt_failed_partial, turns_complete_attempt_cancelled, turns_complete_attempt_failed, turns_recover_orphaned_attempts, migrations_migrations_list [EXTRACTED 0.95]
- **Provider request policy resolution pipeline** — policy_validate_message, policy_resolve_execution_profile, routing_build_provider_messages, capabilities_find_model, openrouter_stream_completion [EXTRACTED 0.90]
- **Backend boundary validation of untrusted renderer input** — policy_validate_message, attachment_budget_check, command_policy_policy_check, artifact_sandbox_sanitize [INFERRED 0.85]
- **Shell surface persistence round-trip: AppState, ShellPreferenceStore, IPC inner handlers, and tests** — app_state_appstate, app_state_surface, storage_sqlite_shellpreferencestore, ipc_app_shell_get_active_surface_inner, ipc_app_shell_set_active_surface_inner, tests_app_shell_shell_surface_commands_keep_lock_order [EXTRACTED 0.95]
- **Workspace shell composition: rail navigation, content panel, and status region working together as tab-list pattern** — components_workspaceshell_workspaceshell, components_surfacerail_surfacerail, components_surfacepanel_surfacepanel, components_statusregion_statusregion, stores_surface_surfacestore [INFERRED 0.85]
- **Chat surface streaming flow: input, message rendering, streaming bubble, chat IPC, and chat store working together** — surfaces_chatsurface_chatsurface, chat_chatinput_chatinput, chat_chatmessage_chatmessage, chat_streamingbubble_streamingbubble, stores_chat_chatstore, api_chat_chatsend, api_chat_chatevent [INFERRED 0.85]
- **Backend-owned reactive stores with optimistic update / rollback pattern** — stores_surface_setsurface, stores_history_deleteconversation, stores_settings_createprivacystore, stores_history_createhistorystore [INFERRED 0.85]
- **Agent Issue Protocol family of structured issue templates** — issue_template_01_bug, issue_template_anomaly, issue_template_architecture, issue_template_dead_code, issue_template_security, issue_template_tech_debt, issue_template_test_gap [EXTRACTED 1.00]
- **Chat send/stream/artifact event handling flow** — stores_chat_handleevent, stores_chat_submitturn, stores_artifacts_receiveartifact, stores_history_setactiveconversationid, stores_chat_chatmessagestate [INFERRED 0.85]
- **Phase 01 App Shell Plan-Review-Fix-Verify Lifecycle** — milestone_01_01_plan_app_shell_bootstrap_plan, milestone_01_02_plan_accessible_shell_plan, milestone_01_review_review_report, milestone_01_review_fix_review_fix_report, milestone_01_03_plan_bootstrap_wiring_plan, milestone_01_verification_app_shell_verification, milestone_01_human_uat_app_shell_uat [EXTRACTED 1.00]
- **Three Runtime Blockers Closed by Gap-Closure Plan 01-03** — milestone_01_03_plan_cr_01_setup_hook, milestone_01_03_plan_cr_02_capability_permissions, milestone_01_03_plan_cr_04_migrations_contract, milestone_01_03_summary_bootstrap_wiring_summary [EXTRACTED 1.00]
- **Codebase Analysis Document Set** — codebase_architecture_architecture_doc, codebase_concerns_concerns_doc, codebase_conventions_conventions_doc, codebase_integrations_integrations_doc, codebase_stack_stack_doc, codebase_structure_structure_doc, codebase_testing_testing_doc [INFERRED 0.85]
- **Routing Phase Bundle** — 02_routing_02_context_phase_2_routing_context, 02_routing_02_discussion_log_phase_2_routing_discussion_log, 02_routing_02_research_phase_2_routing_research, 02_routing_02_summary_phase_2_routing_summary, 02_routing_plan_phase_2_routing_plan [INFERRED 0.95]
- **History Phase Bundle** — 03_history_03_context_phase_3_history_context, 03_history_03_discussion_log_phase_3_history_discussion_log, 03_history_03_patterns_phase_3_history_pattern_map, 03_history_03_ui_spec_phase_3_history_ui_design_contract [INFERRED 0.95]
- **Privacy Phase Bundle** — 04_privacy_04_01_plan_phase_4_privacy_plan_01, 04_privacy_04_01_summary_phase_4_privacy_plan_01_summary, 04_privacy_04_02_plan_phase_4_privacy_plan_02 [INFERRED 0.95]
- **Phase 4 privacy primitives** — command_policy, file_tokens, redaction_pipeline, ipc_privacy, ipc_files, audit_log, settings_surface [EXTRACTED 1.00]
- **Phase 5 artifact stack** — artifact_sandbox, artifact_store, artifact_ready, artifacts_surface, chat_surface_navigation [EXTRACTED 1.00]
- **Phase 6 Release Gate Pipeline (inventory -> capability -> evidence)** — 06_release_06_context_command_inventory_enforcement, 06_release_06_context_release_capability_split, 06_release_06_context_release_evidence_bundle, 06_release_06_summary_ipc_inventory_rs, 06_release_06_summary_release_evidence_rs [EXTRACTED 0.90]
- **Artifact sandbox belt-and-suspenders (backend sanitize + frontend CSP)** — 05_artifacts_05_context_artifact_sandbox, 05_artifacts_05_context_sandboxed_iframe, 05_artifacts_05_ui_spec_fail_closed_guard, docs_tauri_svelte_v5_artifact_sandboxing [EXTRACTED 0.85]
- **Policy-Constrained Provider Runtime contracts** — docs_architecture_md_providers_policy, docs_architecture_md_validatedmessage, docs_architecture_md_executionprofile, docs_architecture_md_routingdecision, docs_architecture_md_policyreceipt, docs_architecture_md_attachment_budget [EXTRACTED 0.90]
- **Provider routing policy enforcement flow** — providers_capabilities_model_allowlist, providers_policy_resolve_execution_profile, providers_policy_validate_message, providers_routing_build_provider_messages, providers_routing_default_system_prompt [INFERRED 0.85]
- **Threat model mitigated threats mapped to enforcement code** — docs_threat_model_renderer_role_injection, docs_threat_model_unconstrained_model_param_override, docs_threat_model_privacy_silently_downgraded, docs_threat_model_unbounded_attachment_ingestion, docs_threat_model_file_token_map_unbounded_growth [EXTRACTED 1.00]
- **AGENTS.md documentation hierarchy for src-tauri subsystems** — src_tauri_agents_src_tauri_crate, src_agents_src_module_tree, ipc_agents_ipc_subtree, providers_agents_providers_subtree, security_agents_security_subtree, storage_agents_storage_subtree [EXTRACTED 1.00]

## Communities (119 total, 20 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.06
Nodes (49): AtomicU64, AttachmentBudgetError, AttachmentMeta, Channel, File token map unbounded growth threat, chat_cancel(), chat_send(), ChatError (+41 more)

### Community 1 - "Community 1"
Cohesion: 0.09
Nodes (57): CapabilitySelection, IntoIterator, InventoryPaths, AllowlistEnvSnapshot, capability_permissions_from_file(), CapabilityFile, CapabilitySelection, command_inventory_detects_extra_command() (+49 more)

### Community 2 - "Community 2"
Cohesion: 0.06
Nodes (44): ArtifactStore struct, backup module (scaffold placeholder), COMMANDS allowlist, FtsStore struct, MemoryStore struct, Migration struct, MIGRATIONS static list, ValidatedMessage struct (+36 more)

### Community 3 - "Community 3"
Cohesion: 0.10
Nodes (35): Surface (enum), F, Arc, Connection, Mutex, Option, PathBuf, Result (+27 more)

### Community 4 - "Community 4"
Cohesion: 0.12
Nodes (36): MemoryKind enum, VerificationState enum, Row, Arc, Option, Result, Self, SqlitePool (+28 more)

### Community 5 - "Community 5"
Cohesion: 0.09
Nodes (40): FnMut, byte_chunks(), classify_provider_error_code(), drive_byte_stream(), drive_byte_stream_returns_cancelled_when_token_fires(), drive_byte_stream_returns_ok_when_done_sentinel_observed(), drive_byte_stream_returns_truncated_stream_for_empty_input(), drive_byte_stream_returns_truncated_stream_when_done_never_arrives() (+32 more)

### Community 6 - "Community 6"
Cohesion: 0.13
Nodes (26): ArtifactSandboxError, DetectedArtifact, Arc, From, Option, Result, Self, SqlitePool (+18 more)

### Community 7 - "Community 7"
Cohesion: 0.13
Nodes (25): Audit Log, command_policy, CredentialStatus, file_tokens, audit_result(), privacy_clear_provider_key(), privacy_get_credential_status(), privacy_set_provider_key() (+17 more)

### Community 8 - "Community 8"
Cohesion: 0.14
Nodes (30): collect_release_evidence (telemetry), main(), ExitStatus, InventoryReport, AsRef, Option, Path, PathBuf (+22 more)

### Community 9 - "Community 9"
Cohesion: 0.12
Nodes (25): src-tauri/build.rs, get_active_surface(), get_active_surface_inner(), set_active_surface(), set_active_surface_inner(), ShellError, main.rs tauri::generate_context!(), Shell/SQLite lock inversion bug (+17 more)

### Community 10 - "Community 10"
Cohesion: 0.13
Nodes (22): ConversationStore, FtsStore, history_get (tauri command), ConversationDetail, ConversationSummary, history_delete(), history_get(), history_list() (+14 more)

### Community 11 - "Community 11"
Cohesion: 0.08
Nodes (27): main(), Deny-by-inventory capability pattern, capability_permissions_from_file, security/command-inventory.toml, compiled_command_allowlist, diff_report, load_command_inventory, load_main_capability (+19 more)

### Community 12 - "Community 12"
Cohesion: 0.23
Nodes (21): Arc, ArtifactContentType, Option, Result, Self, SqlitePool, String, begin_turn_does_not_duplicate_user_message_across_retries() (+13 more)

### Community 13 - "Community 13"
Cohesion: 0.11
Nodes (25): $lib/stores/artifacts, $lib/stores/chat, $lib/stores/history, $lib/stores/settings, $lib/stores/surface, $lib/components/chat/ChatInput.svelte, $lib/components/chat/ChatMessage.svelte, $lib/components/chat/StreamingBubble.svelte (+17 more)

### Community 14 - "Community 14"
Cohesion: 0.14
Nodes (24): AttachmentBudget struct, Metadata-only inspection invariant (no content read before budget check), accepts_within_budget(), AttachmentBudget, AttachmentBudgetError, AttachmentMeta, check(), empty_attachment_set_is_always_accepted() (+16 more)

### Community 15 - "Community 15"
Cohesion: 0.10
Nodes (22): Unconstrained renderer model/parameter override threat, ExecutionProfile struct, Fail-closed privacy resolution rule, PolicyError enum, PolicyReceipt struct, PrivacyMode enum, RoutingDecision struct, providers::capabilities::MODEL_ALLOWLIST (+14 more)

### Community 16 - "Community 16"
Cohesion: 0.16
Nodes (23): ARTIFACT_CSP constant, ArtifactContentType enum, Chars, Peekable, ArtifactSandboxError, escape_html(), find_case_insensitive(), neutralize_javascript_uris() (+15 more)

### Community 17 - "Community 17"
Cohesion: 0.12
Nodes (21): Client, ChatCompletionRequest struct, ProviderMessage struct, Backend-owned system prompt invariant (D-12), ChatCompletionRequest, ProviderMessage, stream_completion(), assistant_msg() (+13 more)

### Community 18 - "Community 18"
Cohesion: 0.09
Nodes (25): Walking Skeleton, Backend-Owned Credentials, Cancellable Streaming Chat, Channel ChatEvent Streaming Transport, Phase 2 Routing Context, Phase 2 Routing Discussion Log, Phase 2 Routing Research, Phase 2 Routing Summary (+17 more)

### Community 19 - "Community 19"
Cohesion: 0.11
Nodes (25): Coordination prompt pattern, Default prompt template, Developer prompt layer, Execution prompt pattern, Use explicit steps, Add guardrails at decision points, Memory prompt pattern, Make retrieval narrow (+17 more)

### Community 20 - "Community 20"
Cohesion: 0.17
Nodes (19): FileTokenError, basename(), FileReadResponse, files_open_dialog(), files_read_token(), FilesError, FileTokenResponse, safe_metadata() (+11 more)

### Community 21 - "Community 21"
Cohesion: 0.11
Nodes (23): src/app_state.rs (shared runtime state), Policy-Constrained Provider Runtime, Provider Routing, Unsatisfiable privacy requirement silently downgraded threat, Threat Model, Unbounded attachment ingestion threat, ipc/AGENTS.md subtree (frontend-callable commands), ipc/mod.rs (frontend-facing command surface) (+15 more)

### Community 22 - "Community 22"
Cohesion: 0.15
Nodes (20): allowlist_case_strategy(), allowlist_separator_strategy(), allowlist_token_strategy(), AllowlistEnvSnapshot, command_name_strategy(), handler_indent_strategy(), render_generate_handler(), temp_env_guard() (+12 more)

### Community 23 - "Community 23"
Cohesion: 0.14
Nodes (12): MODEL_ALLOWLIST constant, ModelSpec struct, ProviderId enum, DEFAULT_MODEL constant, default_model_spec(), find_model(), ModelSpec, ProviderId (+4 more)

### Community 24 - "Community 24"
Cohesion: 0.15
Nodes (20): ARTF-01: Sandboxed artifact preview, ARTF-02: Stop/reload runaway preview, ARTF-03: Keyboard accessible preview, HIST-01: Save conversations locally, HIST-02: Search prior conversations, HIST-03: Delete/retain history per rules, Requirements: Desktop AI Client, ROUTE-01: Deterministic provider selection (+12 more)

### Community 25 - "Community 25"
Cohesion: 0.16
Nodes (18): artifact_dismiss (tauri command), artifact_get (tauri command), assert_main_window, chat_cancel (tauri command), security::command_policy, basename, files_open_dialog (tauri command), files_read_token (tauri command) (+10 more)

### Community 26 - "Community 26"
Cohesion: 0.19
Nodes (17): default_fixture (telemetry::memory_replay), main(), run_replay (telemetry::memory_replay), MemoryKind, MemoryStore, Arc, Option, SqlitePool (+9 more)

### Community 27 - "Community 27"
Cohesion: 0.23
Nodes (16): FtsStore::search(), SearchResult struct, Arc, Result, Self, SqlitePool, String, Vec (+8 more)

### Community 28 - "Community 28"
Cohesion: 0.23
Nodes (14): ArtifactStore, artifact_dismiss(), artifact_get(), ArtifactError, ArtifactResponse, assert_main_window(), ArtifactContentType, From (+6 more)

### Community 29 - "Community 29"
Cohesion: 0.16
Nodes (17): Migrations as Embedded Rust Constants (open question), 01-01-PLAN: App Shell Bootstrap, 01-01-SUMMARY: App Shell Bootstrap Summary, SAVEPOINT-based Migration Runner Decision, ShellPreferenceStore UPSERT Decision, 01-02-PLAN: Accessible Workspace Shell and Smoke Coverage, 01-03-PLAN: Bootstrap Wiring and Correctness Fixes (Gap Closure), CR-01: Missing .setup() Hook (+9 more)

### Community 30 - "Community 30"
Cohesion: 0.12
Nodes (17): devDependencies, eslint, @eslint/js, eslint-plugin-svelte, globals, prettier, prettier-plugin-svelte, svelte-check (+9 more)

### Community 31 - "Community 31"
Cohesion: 0.17
Nodes (15): security::artifact_sandbox, SQLite artifacts table, ArtifactsSurface.svelte, ipc::artifacts command surface, Sandboxed iframe (srcdoc), ShellError pattern, Tauri WebviewWindow isolation (deferred), Artifact preview keyboard/screen-reader smoke test (+7 more)

### Community 32 - "Community 32"
Cohesion: 0.18
Nodes (11): set_active_surface (tauri command), set_active_surface_inner, HashMap, SecretsState, ShellState, AppState, CancellationToken, Mutex (+3 more)

### Community 33 - "Community 33"
Cohesion: 0.24
Nodes (7): ArtifactReady, Artifact Sandbox, Artifact Store, ArtifactsSurface, ChatSurface navigation, Phase 04 Privacy, Phase 05 Artifacts

### Community 34 - "Community 34"
Cohesion: 0.20
Nodes (15): Lock Ordering Invariant (shell before sqlite), AppShell.svelte Dead Code, CR-01: Double SurfaceRail Render, CR-02: Svelte 4 slot vs Svelte 5 snippet Mismatch, CR-03: Race Condition in get_active_surface, CR-04: SQL Injection Precedent in Migration Savepoint, Fix: Remove AppShell from Layout (Double Rail), Fix: Migration ID Validation Safety Comment (+7 more)

### Community 35 - "Community 35"
Cohesion: 0.19
Nodes (10): PolicyReceipt, PrivacyMode, Role, ValidatedMessage, Default, Err, FromStr, Result (+2 more)

### Community 36 - "Community 36"
Cohesion: 0.31
Nodes (11): chatCancel(), ChatEvent, ChatMessage, chatSend(), ChatSendParams, Strictly increasing sequence per attempt stream protocol invariant, ChatMessageState, chatStore (+3 more)

### Community 37 - "Community 37"
Cohesion: 0.17
Nodes (12): receiveArtifact, handleEvent, ConversationDetail, createHistoryStore(), historyStore, load (historyStore), loadConversation, search (historyStore) (+4 more)

### Community 38 - "Community 38"
Cohesion: 0.18
Nodes (12): ArtifactReady channel event, artifacts::detector, chat_send attachment token resolution, ChatEvent tagged enum, file_tokens::resolve_token, security::attachment_budget, Policy-Constrained Provider Runtime, Privacy Boundaries (+4 more)

### Community 39 - "Community 39"
Cohesion: 0.26
Nodes (11): security/command-inventory.toml, ipc::inventory (inventory.rs), security/release-capabilities.toml, bin/verify-command-inventory, T-1: Reviewed inventory + capability catalog, T-2: Command-inventory verifier, AGENTS.md (root intent layer), Universal Agent Constitution (+3 more)

### Community 40 - "Community 40"
Cohesion: 0.17
Nodes (12): chat_send (tauri command), ChatError (enum), ChatEvent (enum), Conversation Transaction Protocol, read_attachment, resolve_attachments, run_stream, title_from_content (+4 more)

### Community 41 - "Community 41"
Cohesion: 0.26
Nodes (12): Agent Issue Protocol, Credential Advisory Gate / Security Advisory bypass, Delete, Don't Comment directive, FSV-Style State Delta Assertion, Bug Report issue template, Anomaly issue template, Architecture issue template, Dead Code issue template (+4 more)

### Community 42 - "Community 42"
Cohesion: 0.17
Nodes (12): scripts, build, check, check:watch, dev, format, format:write, frontend:build (+4 more)

### Community 43 - "Community 43"
Cohesion: 0.17
Nodes (11): compilerOptions, allowJs, checkJs, esModuleInterop, forceConsistentCasingInFileNames, resolveJsonModule, skipLibCheck, sourceMap (+3 more)

### Community 44 - "Community 44"
Cohesion: 0.38
Nodes (9): artifactDismiss(), artifactGet(), ArtifactPreviewResponse, ArtifactContentType, artifactsStore, ArtifactState, createArtifactsStore(), handleReload (+1 more)

### Community 45 - "Community 45"
Cohesion: 0.24
Nodes (11): ChatInput.svelte component, ChatMessage.svelte component, D-06: amber Interrupted/Cancelled badge styling, D-05: hybrid loading UX (skeleton to streaming-content transition), StreamingBubble.svelte component, SearchBar.svelte component, T-03-18: clear debounce timer on unmount to prevent post-unmount invocation, ArtifactsSurface.svelte component (+3 more)

### Community 46 - "Community 46"
Cohesion: 0.25
Nodes (9): Error, AppHandle, Result, Self, String, audit_entry_contains_only_metadata_fields(), audit_entry_round_trips_json(), AuditEntry (+1 more)

### Community 47 - "Community 47"
Cohesion: 0.40
Nodes (8): Arc, Result, Self, SqlitePool, delete_conversation_removes_conversation_and_messages(), delete_nonexistent_conversation_is_noop(), in_memory_pool(), RetentionStore

### Community 48 - "Community 48"
Cohesion: 0.22
Nodes (10): Command Inventory Enforcement (deny-by-inventory), Release/dev capability split, Deny-by-inventory command exposure, FTS5 External-Content Schema, FTS5 Query Safety, Tauri/Svelte AI App Architecture (Adversarial Hardened v5), Explicit release capability selection, Real SSE Parsing Contract (+2 more)

### Community 49 - "Community 49"
Cohesion: 0.29
Nodes (7): normalizeIpcError(), createPrivacyStore(), CredentialStatus, ProviderId, createSurfaceStore(), Surface, SURFACE_LABELS

### Community 50 - "Community 50"
Cohesion: 0.24
Nodes (7): CI workflow (ci.yml), backend CI job (matrix os), frontend CI job, rust-audit CI job, Default PULL_REQUEST_TEMPLATE.md, Release PR template, config

### Community 51 - "Community 51"
Cohesion: 0.24
Nodes (10): Architecture Document, Single Command Policy Authority Pattern, Opaque File Token Pattern, Six-Source Command Inventory Reconciliation, Codebase Concerns Document, Documentation Drift Issue (issue #4), GTK/WebKit Toolchain Requirement Gap, ipc::providers.rs Unregistered Stub (issue #5) (+2 more)

### Community 52 - "Community 52"
Cohesion: 0.20
Nodes (10): Layered Tauri/Svelte Desktop Client Pattern, Technology Stack Document, Codebase Structure Document, Core Value: Privacy-First Local History with Routed Inference, Desktop AI Client Project, Least-Privilege Architecture Decision, REL-01: Reviewed command inventory, REL-02: Release evidence bundle (+2 more)

### Community 53 - "Community 53"
Cohesion: 0.29
Nodes (9): AppShell.svelte component, StatusRegion.svelte component, SurfacePanel.svelte component, SurfaceRail.svelte component, Roving tabindex tab-list ARIA pattern, WorkspaceShell.svelte component, hydrate (surfaceStore), surfaceStore (+1 more)

### Community 54 - "Community 54"
Cohesion: 0.33
Nodes (10): Agent Context (memory-first vocabulary), Evidence-Gated Memory Engine (Phase 1, shadow mode), telemetry::memory_replay, storage::memory::MemoryStore, Design Blueprint, Implementation Plan (memory-loop milestones), Memory Loop design doc, promotion_rule (deterministic judge) (+2 more)

### Community 55 - "Community 55"
Cohesion: 0.20
Nodes (10): Fixture Families (adversarial-sse, provider-drift, fts-query-abuse, srcdoc-escaping, wal-recovery, capability-drift), Release Evidence (doc), release-evidence/test-runs/cargo-test.log, collect-release-evidence binary, release-evidence/fixtures.toml, release-evidence/manifest.toml, Release Evidence snapshot 2026-06-19T23:40:14Z, release-evidence/README.md (+2 more)

### Community 56 - "Community 56"
Cohesion: 0.25
Nodes (9): Renderer role injection threat (hostile renderer), Role enum, PolicyError::InvalidRole, validate_message(), validate_message_preserves_empty_content(), validate_message_rejects_arbitrary_role_strings(), validate_message_rejects_system_role_injection(), providers::routing::DEFAULT_SYSTEM_PROMPT (+1 more)

### Community 57 - "Community 57"
Cohesion: 0.29
Nodes (6): Display, Formatter, Surface, Err, FromStr, Result

### Community 58 - "Community 58"
Cohesion: 0.25
Nodes (8): Conversation Transaction Protocol, ExecutionProfile, PolicyReceipt, providers::policy, RoutingDecision, storage::turns::TurnStore, ValidatedMessage / Role, tauri::ipc::Channel<StreamEvent> streaming

### Community 59 - "Community 59"
Cohesion: 0.25
Nodes (7): dependencies, @tauri-apps/api, name, packageManager, private, type, version

### Community 60 - "Community 60"
Cohesion: 0.29
Nodes (7): Release evidence bundle (first-pass), bin/collect-release-evidence, telemetry::release_evidence, T-3: Release evidence bundle + fixtures, ipc::inventory::verify_inventory, v1.0 Milestone Audit, Phase verification chain (blocked)

### Community 61 - "Community 61"
Cohesion: 0.33
Nodes (4): app_state_initializes_active_requests_empty(), app_state_initializes_file_tokens_empty(), surface_default_is_chat(), Self

### Community 62 - "Community 62"
Cohesion: 0.29
Nodes (7): buildHistoryAndNewMessage, Conversation Transaction Protocol, hydrate (chatStore), retryMessage, sendMessage, submitTurn, MessageSummary

### Community 63 - "Community 63"
Cohesion: 0.33
Nodes (6): Keyring Secret Store, Phase 4 Privacy Plan 01, Phase 4 Privacy Plan 01 Summary, Command Policy Allow Table, Phase 4 Privacy Plan 02, Redaction Primitives

### Community 64 - "Community 64"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 65 - "Community 65"
Cohesion: 0.47
Nodes (6): capability_hash(), capability_hash_is_deterministic(), ExecutionProfile, RoutingDecision, ProviderId, String

### Community 66 - "Community 66"
Cohesion: 0.40
Nodes (5): In-Memory SQLite Test Helper Pattern, Testing Patterns Document, 01-02-SUMMARY: Accessible Workspace Shell Summary, Cargo Integration Test Location Decision, Roving Tabindex ARIA Pattern Decision

### Community 67 - "Community 67"
Cohesion: 0.50
Nodes (5): ConversationList.svelte component, ConversationRow.svelte component, T-03-16: render FTS5 snippet markers as plain text, not @html, to prevent stored-content XSS, relativeTime, ConversationSummary

### Community 68 - "Community 68"
Cohesion: 0.40
Nodes (4): overrides, plugins, singleQuote, useTabs

### Community 69 - "Community 69"
Cohesion: 0.50
Nodes (4): Backend-owned credentials policy (D-10): frontend never constructs api_key, tauri.conf.json app configuration, bundle.icon path (icons/icon.ico), Content Security Policy (default-src self, connect-src ipc)

### Community 70 - "Community 70"
Cohesion: 0.50
Nodes (4): get_active_surface (tauri command), get_active_surface_inner, ShellState, Surface

### Community 72 - "Community 72"
Cohesion: 0.50
Nodes (4): SecretsState, Default, Option, SecretString

### Community 73 - "Community 73"
Cohesion: 0.50
Nodes (4): clearProviderKey, setProviderKey, handleRemoveConfirm, handleSave

### Community 74 - "Community 74"
Cohesion: 0.67
Nodes (3): Frontend IPC Wrapper Boundary, Frontend Presentation Components, Frontend Reactive State

### Community 75 - "Community 75"
Cohesion: 0.67
Nodes (3): Coding Conventions Document, Optimistic Update with Rollback Pattern, Svelte Store Factory Pattern

### Community 76 - "Community 76"
Cohesion: 0.67
Nodes (3): Issue 13 (PR Created, PR #18, src-tauri/src/storage/fts.rs), Issue Processing State table, src-tauri/src/storage/fts.rs

### Community 77 - "Community 77"
Cohesion: 0.67
Nodes (3): providers::openrouter, ChatEvent::Done.model, providers::sse

### Community 78 - "Community 78"
Cohesion: 0.67
Nodes (3): PR Contract GitHub Workflow, PR Evidence Contract Validation, validate-pr-body Job

## Ambiguous Edges - Review These
- `ArtifactReady channel event` → `chat_send attachment token resolution`  [AMBIGUOUS]
  .planning/milestones/v1.0-phases/05-artifacts/05-CONTEXT.md · relation: references
- `Provider Routing` → `ipc/AGENTS.md subtree (frontend-callable commands)`  [AMBIGUOUS]
  src-tauri/src/ipc/AGENTS.md · relation: references
- `storage::memory::bounded_retrieve` → `ipc/mod.rs (frontend-facing command surface)`  [AMBIGUOUS]
  docs/threat-model.md · relation: references

## Knowledge Gaps
- **303 isolated node(s):** `useTabs`, `singleQuote`, `plugins`, `overrides`, `name` (+298 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **20 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What is the exact relationship between `ArtifactReady channel event` and `chat_send attachment token resolution`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **What is the exact relationship between `Provider Routing` and `ipc/AGENTS.md subtree (frontend-callable commands)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **What is the exact relationship between `storage::memory::bounded_retrieve` and `ipc/mod.rs (frontend-facing command surface)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **Why does `command_policy` connect `Community 7` to `Community 0`, `Community 9`, `Community 10`, `Community 1`?**
  _High betweenness centrality (0.153) - this node is a cross-community bridge._
- **Why does `run_migrations()` connect `Community 2` to `Community 3`, `Community 4`, `Community 6`, `Community 12`, `Community 47`, `Community 26`, `Community 27`?**
  _High betweenness centrality (0.137) - this node is a cross-community bridge._
- **Why does `Threat Model` connect `Community 21` to `Community 0`, `Community 65`, `Community 35`, `Community 15`, `Community 19`, `Community 23`, `Community 56`?**
  _High betweenness centrality (0.085) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `resolve_execution_profile()` (e.g. with `check()` and `Add guardrails at decision points`) actually correct?**
  _`resolve_execution_profile()` has 2 INFERRED edges - model-reasoned connections that need verification._