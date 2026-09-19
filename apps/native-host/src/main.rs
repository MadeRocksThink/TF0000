use std::{
    env,
    io::{self, Read, Write},
    path::PathBuf,
};

use context_core::{
    AppendBranchVersionInput, ApplyMemoryUpdateInput, AskMemoryInput, CaptureConversationInput,
    ComposeContextInput, ContextStore, CreateAgentWriteCandidateInput, CreateMemoryBranchInput,
    CreateTemporaryAttachmentInput, GetDecisionsInput, HandoffInput, MapWorkspaceInput,
    PreviewMemoryUpdateInput, ProjectContextInput, RecordMcpAuditInput, RegisterMcpClientInput,
    ReviewAgentWriteCandidateInput, SaveCodeReferenceInput, SaveContextPackInput, SearchChatsInput,
    SearchDecisionsInput, SearchInput, SearchMemoriesInput, SecretScanInput, SetMcpPermissionInput,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

const PROTOCOL_VERSION: u8 = 1;
const MAX_FRAME_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum IncomingMessage {
    Hello {
        protocol_version: u8,
        request_id: String,
        extension_id: String,
        nonce: String,
    },
    Request {
        protocol_version: u8,
        request_id: String,
        session_id: String,
        action: String,
        #[serde(default)]
        payload: Value,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutgoingMessage {
    protocol_version: u8,
    request_id: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConversationReferencePayload {
    provider: String,
    external_ref: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConversationPayload {
    conversation_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BindingPayload {
    conversation_id: String,
    target_type: String,
    target_id: String,
    enabled: bool,
    lifetime_mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MemoryPayload {
    memory_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RestoreVersionPayload {
    memory_id: String,
    version_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FinalizeBranchPayload {
    branch_id: String,
    mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResolveConflictPayload {
    conflict_id: String,
    resolution: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkspacePayload {
    workspace_uri: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectPayload {
    project_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CandidateStatusPayload {
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PackPayload {
    pack_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct McpToolCallPayload {
    client_id: String,
    tool_name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct McpAuditLogPayload {
    client_id: String,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct McpSaveCandidatePayload {
    project_id: String,
    memory_type: String,
    title: String,
    content: String,
}

struct McpAccessTarget {
    project_id: Option<String>,
    access_type: &'static str,
    entity_type: Option<&'static str>,
    entity_id: Option<String>,
}

struct HostSession {
    session_id: Option<String>,
    store: ContextStore,
}

impl HostSession {
    fn new(store: ContextStore) -> Self {
        Self {
            session_id: None,
            store,
        }
    }

    fn handle(&mut self, message: IncomingMessage) -> OutgoingMessage {
        match message {
            IncomingMessage::Hello {
                protocol_version,
                request_id,
                extension_id,
                nonce,
            } => {
                if protocol_version != PROTOCOL_VERSION {
                    return failure(request_id, "unsupported protocol version");
                }
                if !valid_extension_id(&extension_id) || nonce.len() < 16 || nonce.len() > 200 {
                    return failure(request_id, "invalid extension handshake");
                }
                let session_id = Uuid::new_v4().to_string();
                self.session_id = Some(session_id.clone());
                success(
                    request_id,
                    Some(session_id.clone()),
                    json!({ "sessionId": session_id, "nonce": nonce }),
                )
            }
            IncomingMessage::Request {
                protocol_version,
                request_id,
                session_id,
                action,
                payload,
            } => {
                if protocol_version != PROTOCOL_VERSION {
                    return failure(request_id, "unsupported protocol version");
                }
                if self.session_id.as_deref() != Some(&session_id) {
                    return failure(request_id, "unauthenticated native messaging session");
                }
                match self.handle_action(&action, payload) {
                    Ok(data) => success(request_id, Some(session_id), data),
                    Err(error) => failure(request_id, &error),
                }
            }
        }
    }

    fn handle_action(&self, action: &str, payload: Value) -> Result<Value, String> {
        match action {
            "health" => serde_json::to_value(self.store.health().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string()),
            "dashboard" => serde_json::to_value(self.store.snapshot().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string()),
            "sources" => serde_json::to_value(
                self.store
                    .list_captured_sources()
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string()),
            "search" => {
                let input: SearchInput =
                    serde_json::from_value(payload).map_err(|e| format!("invalid search: {e}"))?;
                serde_json::to_value(self.store.search(input).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())
            }
            "askMemory" => {
                let input: AskMemoryInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid memory question: {e}"))?;
                serde_json::to_value(self.store.ask_memory(input).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())
            }
            "detectSecrets" => {
                let input: SecretScanInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid secret scan: {e}"))?;
                serde_json::to_value(self.store.detect_secrets(&input.text))
                    .map_err(|e| e.to_string())
            }
            "packs" => serde_json::to_value(
                self.store
                    .list_context_pack_details()
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string()),
            "capture" => {
                let input: CaptureConversationInput =
                    serde_json::from_value(payload).map_err(|e| format!("invalid capture: {e}"))?;
                serde_json::to_value(
                    self.store
                        .capture_conversation(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "compose" => {
                let input: ComposeContextInput =
                    serde_json::from_value(payload).map_err(|e| format!("invalid compose: {e}"))?;
                serde_json::to_value(
                    self.store
                        .compose_context(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "previewHandoff" => {
                let input: HandoffInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid handoff preview: {e}"))?;
                serde_json::to_value(
                    self.store
                        .preview_handoff(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "recordHandoff" => {
                let input: HandoffInput =
                    serde_json::from_value(payload).map_err(|e| format!("invalid handoff: {e}"))?;
                serde_json::to_value(
                    self.store
                        .record_handoff(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "savePack" => {
                let input: SaveContextPackInput =
                    serde_json::from_value(payload).map_err(|e| format!("invalid pack: {e}"))?;
                serde_json::to_value(
                    self.store
                        .save_context_pack(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "previewMemoryUpdate" => {
                let input: PreviewMemoryUpdateInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid memory update preview: {e}"))?;
                serde_json::to_value(
                    self.store
                        .preview_memory_update(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "applyMemoryUpdate" => {
                let input: ApplyMemoryUpdateInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid memory update: {e}"))?;
                serde_json::to_value(
                    self.store
                        .apply_memory_update(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "memoryHistory" => {
                let input: MemoryPayload =
                    serde_json::from_value(payload).map_err(|e| format!("invalid memory: {e}"))?;
                serde_json::to_value(
                    self.store
                        .memory_history(&input.memory_id)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "restoreMemoryVersion" => {
                let input: RestoreVersionPayload = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid version restore: {e}"))?;
                serde_json::to_value(
                    self.store
                        .restore_memory_version(&input.memory_id, &input.version_id)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "createMemoryBranch" => {
                let input: CreateMemoryBranchInput =
                    serde_json::from_value(payload).map_err(|e| format!("invalid branch: {e}"))?;
                serde_json::to_value(
                    self.store
                        .create_memory_branch(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "appendBranchVersion" => {
                let input: AppendBranchVersionInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid branch version: {e}"))?;
                serde_json::to_value(
                    self.store
                        .append_branch_version(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "memoryBranches" => {
                let memory_id = if payload.is_null() {
                    None
                } else {
                    Some(
                        serde_json::from_value::<MemoryPayload>(payload)
                            .map_err(|e| format!("invalid memory: {e}"))?
                            .memory_id,
                    )
                };
                serde_json::to_value(
                    self.store
                        .list_memory_branches(memory_id.as_deref())
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "finalizeMemoryBranch" => {
                let input: FinalizeBranchPayload = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid branch finalization: {e}"))?;
                serde_json::to_value(
                    self.store
                        .finalize_memory_branch(&input.branch_id, &input.mode)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "memoryConflicts" => serde_json::to_value(
                self.store
                    .list_memory_conflicts(None, "unresolved")
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string()),
            "resolveMemoryConflict" => {
                let input: ResolveConflictPayload = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid conflict resolution: {e}"))?;
                self.store
                    .resolve_memory_conflict(&input.conflict_id, &input.resolution)
                    .map_err(|e| e.to_string())?;
                Ok(json!({ "resolved": true }))
            }
            "workspaceMapping" => {
                let input: WorkspacePayload = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid workspace: {e}"))?;
                serde_json::to_value(
                    self.store
                        .workspace_mapping(&input.workspace_uri)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "mapWorkspace" => {
                let input: MapWorkspaceInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid workspace mapping: {e}"))?;
                serde_json::to_value(self.store.map_workspace(input).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())
            }
            "saveCodeReference" => {
                let input: SaveCodeReferenceInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid code reference: {e}"))?;
                serde_json::to_value(
                    self.store
                        .save_code_reference(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "codeReferences" => {
                let input: ProjectPayload =
                    serde_json::from_value(payload).map_err(|e| format!("invalid project: {e}"))?;
                serde_json::to_value(
                    self.store
                        .list_code_references(&input.project_id)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "getProjectContext" => {
                let input: ProjectContextInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid project context request: {e}"))?;
                serde_json::to_value(
                    self.store
                        .get_project_context(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "searchDecisions" => {
                let input: SearchDecisionsInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid decision search: {e}"))?;
                serde_json::to_value(
                    self.store
                        .search_decisions(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "createAgentWriteCandidate" => {
                let input: CreateAgentWriteCandidateInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid write candidate: {e}"))?;
                serde_json::to_value(
                    self.store
                        .create_agent_write_candidate(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "agentWriteCandidates" => {
                let input: CandidateStatusPayload = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid candidate status: {e}"))?;
                serde_json::to_value(
                    self.store
                        .agent_write_candidates(input.status.as_deref())
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "reviewAgentWriteCandidate" => {
                let input: ReviewAgentWriteCandidateInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid candidate review: {e}"))?;
                serde_json::to_value(
                    self.store
                        .review_agent_write_candidate(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "registerMcpClient" => {
                let input: RegisterMcpClientInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid MCP client: {e}"))?;
                serde_json::to_value(
                    self.store
                        .register_mcp_client(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "mcpPermission" => {
                let input: SetMcpPermissionInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid MCP permission: {e}"))?;
                serde_json::to_value(
                    self.store
                        .mcp_permission(&input.client_id, input.project_id.as_deref())
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "setMcpPermission" => {
                let input: SetMcpPermissionInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid MCP permission: {e}"))?;
                serde_json::to_value(
                    self.store
                        .set_mcp_permission(input)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "mcpAuditLog" => {
                let input: McpAuditLogPayload = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid MCP audit request: {e}"))?;
                serde_json::to_value(
                    self.store
                        .mcp_audit_log(&input.client_id, input.limit.unwrap_or(100))
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "mcpToolCall" => {
                let input: McpToolCallPayload = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid MCP tool call: {e}"))?;
                self.handle_mcp_tool_call(input)
            }
            "conversationContextByRef" => {
                let input: ConversationReferencePayload = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid conversation reference: {e}"))?;
                serde_json::to_value(
                    self.store
                        .conversation_context_by_reference(&input.provider, &input.external_ref)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "setBinding" => {
                let input: BindingPayload =
                    serde_json::from_value(payload).map_err(|e| format!("invalid binding: {e}"))?;
                self.store
                    .set_context_binding(
                        &input.conversation_id,
                        &input.target_type,
                        &input.target_id,
                        input.enabled,
                        &input.lifetime_mode,
                    )
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(
                    self.store
                        .conversation_context(&input.conversation_id)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "attachTemporary" => {
                let input: CreateTemporaryAttachmentInput = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid temporary attachment: {e}"))?;
                let conversation_id = input.conversation_id.clone();
                self.store
                    .create_temporary_attachment(input)
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(
                    self.store
                        .conversation_context(&conversation_id)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "clearTemporary" => {
                let input: ConversationPayload = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid conversation: {e}"))?;
                self.store
                    .clear_temporary_context(&input.conversation_id)
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(
                    self.store
                        .conversation_context(&input.conversation_id)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            "consumePrompt" => {
                let input: ConversationPayload = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid conversation: {e}"))?;
                self.store
                    .consume_prompt(&input.conversation_id)
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(
                    self.store
                        .conversation_context(&input.conversation_id)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
            }
            _ => Err(format!("unsupported action: {action}")),
        }
    }

    fn handle_mcp_tool_call(&self, input: McpToolCallPayload) -> Result<Value, String> {
        let McpAccessTarget {
            project_id,
            access_type,
            entity_type,
            entity_id,
        } = self.mcp_access_target(&input)?;
        let permission = self
            .store
            .mcp_permission(&input.client_id, project_id.as_deref())
            .map_err(|error| error.to_string())?;
        let allowed = if access_type == "read" {
            permission.read_allowed
        } else {
            permission.candidate_write_allowed
        };
        if !allowed {
            self.record_mcp_audit(
                &input,
                access_type,
                project_id,
                entity_type,
                entity_id,
                "denied",
            )?;
            return Err(format!(
                "MCP client '{}' is not allowed to perform {} operations for this project",
                input.client_id, access_type
            ));
        }

        let result = self.execute_mcp_tool(&input);
        let outcome = if result.is_ok() { "allowed" } else { "error" };
        self.record_mcp_audit(
            &input,
            access_type,
            project_id,
            entity_type,
            entity_id,
            outcome,
        )?;
        result
    }

    fn mcp_access_target(&self, input: &McpToolCallPayload) -> Result<McpAccessTarget, String> {
        match input.tool_name.as_str() {
            "search_memory" => {
                let arguments: SearchMemoriesInput =
                    serde_json::from_value(input.arguments.clone())
                        .map_err(|error| format!("invalid search_memory arguments: {error}"))?;
                Ok(McpAccessTarget {
                    project_id: Some(arguments.project_id),
                    access_type: "read",
                    entity_type: Some("project"),
                    entity_id: None,
                })
            }
            "get_memory" => {
                let arguments: MemoryPayload = serde_json::from_value(input.arguments.clone())
                    .map_err(|error| format!("invalid get_memory arguments: {error}"))?;
                Ok(McpAccessTarget {
                    project_id: self
                        .store
                        .memory_project_id(&arguments.memory_id)
                        .map_err(|error| error.to_string())?,
                    access_type: "read",
                    entity_type: Some("memory"),
                    entity_id: Some(arguments.memory_id),
                })
            }
            "get_project_context" => {
                let arguments: ProjectContextInput =
                    serde_json::from_value(input.arguments.clone()).map_err(|error| {
                        format!("invalid get_project_context arguments: {error}")
                    })?;
                Ok(McpAccessTarget {
                    project_id: Some(arguments.project_id),
                    access_type: "read",
                    entity_type: Some("project"),
                    entity_id: None,
                })
            }
            "get_decisions" => {
                let arguments: GetDecisionsInput = serde_json::from_value(input.arguments.clone())
                    .map_err(|error| format!("invalid get_decisions arguments: {error}"))?;
                Ok(McpAccessTarget {
                    project_id: Some(arguments.project_id),
                    access_type: "read",
                    entity_type: Some("project"),
                    entity_id: None,
                })
            }
            "get_context_pack" => {
                let arguments: PackPayload = serde_json::from_value(input.arguments.clone())
                    .map_err(|error| format!("invalid get_context_pack arguments: {error}"))?;
                Ok(McpAccessTarget {
                    project_id: self
                        .store
                        .context_pack_project_id(&arguments.pack_id)
                        .map_err(|error| error.to_string())?,
                    access_type: "read",
                    entity_type: Some("context_pack"),
                    entity_id: Some(arguments.pack_id),
                })
            }
            "search_chats" => {
                let arguments: SearchChatsInput =
                    serde_json::from_value(input.arguments.clone())
                        .map_err(|error| format!("invalid search_chats arguments: {error}"))?;
                Ok(McpAccessTarget {
                    project_id: Some(arguments.project_id),
                    access_type: "read",
                    entity_type: Some("project"),
                    entity_id: None,
                })
            }
            "save_memory_candidate" => {
                let arguments: McpSaveCandidatePayload =
                    serde_json::from_value(input.arguments.clone()).map_err(|error| {
                        format!("invalid save_memory_candidate arguments: {error}")
                    })?;
                Ok(McpAccessTarget {
                    project_id: Some(arguments.project_id),
                    access_type: "candidate_write",
                    entity_type: Some("project"),
                    entity_id: None,
                })
            }
            _ => Err(format!("unknown MCP tool: {}", input.tool_name)),
        }
    }

    fn execute_mcp_tool(&self, input: &McpToolCallPayload) -> Result<Value, String> {
        match input.tool_name.as_str() {
            "search_memory" => {
                let arguments =
                    serde_json::from_value::<SearchMemoriesInput>(input.arguments.clone())
                        .map_err(|error| error.to_string())?;
                serde_json::to_value(
                    self.store
                        .search_memories(arguments)
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())
            }
            "get_memory" => {
                let arguments = serde_json::from_value::<MemoryPayload>(input.arguments.clone())
                    .map_err(|error| error.to_string())?;
                serde_json::to_value(
                    self.store
                        .get_memory(&arguments.memory_id)
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())
            }
            "get_project_context" => {
                let arguments =
                    serde_json::from_value::<ProjectContextInput>(input.arguments.clone())
                        .map_err(|error| error.to_string())?;
                serde_json::to_value(
                    self.store
                        .get_project_context(arguments)
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())
            }
            "get_decisions" => {
                let arguments =
                    serde_json::from_value::<GetDecisionsInput>(input.arguments.clone())
                        .map_err(|error| error.to_string())?;
                serde_json::to_value(
                    self.store
                        .get_decisions(arguments)
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())
            }
            "get_context_pack" => {
                let arguments = serde_json::from_value::<PackPayload>(input.arguments.clone())
                    .map_err(|error| error.to_string())?;
                let detail = self
                    .store
                    .get_context_pack(&arguments.pack_id)
                    .map_err(|error| error.to_string())?;
                let content = self
                    .store
                    .compose_context(ComposeContextInput {
                        memory_ids: Vec::new(),
                        memory_space_ids: Vec::new(),
                        context_pack_ids: vec![arguments.pack_id],
                        sources: Vec::new(),
                        conversation_id: None,
                    })
                    .map_err(|error| error.to_string())?;
                Ok(json!({ "detail": detail, "content": content }))
            }
            "search_chats" => {
                let arguments = serde_json::from_value::<SearchChatsInput>(input.arguments.clone())
                    .map_err(|error| error.to_string())?;
                serde_json::to_value(
                    self.store
                        .search_chats(arguments)
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())
            }
            "save_memory_candidate" => {
                let arguments =
                    serde_json::from_value::<McpSaveCandidatePayload>(input.arguments.clone())
                        .map_err(|error| error.to_string())?;
                serde_json::to_value(
                    self.store
                        .create_agent_write_candidate(CreateAgentWriteCandidateInput {
                            project_id: arguments.project_id,
                            requested_by: format!("mcp:{}", input.client_id),
                            memory_type: arguments.memory_type,
                            title: arguments.title,
                            content: arguments.content,
                        })
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())
            }
            _ => Err(format!("unknown MCP tool: {}", input.tool_name)),
        }
    }

    fn record_mcp_audit(
        &self,
        input: &McpToolCallPayload,
        access_type: &str,
        project_id: Option<String>,
        entity_type: Option<&str>,
        entity_id: Option<String>,
        outcome: &str,
    ) -> Result<(), String> {
        self.store
            .record_mcp_audit(RecordMcpAuditInput {
                client_id: input.client_id.clone(),
                tool_name: input.tool_name.clone(),
                access_type: access_type.into(),
                project_id,
                entity_type: entity_type.map(str::to_owned),
                entity_id,
                outcome: outcome.into(),
            })
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

fn success(request_id: String, session_id: Option<String>, data: Value) -> OutgoingMessage {
    OutgoingMessage {
        protocol_version: PROTOCOL_VERSION,
        request_id,
        ok: true,
        session_id,
        data: Some(data),
        error: None,
    }
}

fn failure(request_id: String, message: &str) -> OutgoingMessage {
    OutgoingMessage {
        protocol_version: PROTOCOL_VERSION,
        request_id,
        ok: false,
        session_id: None,
        data: None,
        error: Some(message.to_owned()),
    }
}

fn valid_extension_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|value| (b'a'..=b'p').contains(&value))
}

fn database_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("TF0000_CONTEXT_DB") {
        return Ok(PathBuf::from(path));
    }
    #[cfg(target_os = "windows")]
    let root = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let root = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|path| path.join("Library/Application Support"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let root = env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".local/share"))
    });
    let root = root.ok_or_else(|| "unable to locate the application data directory".to_string())?;
    let database = root.join("com.tf0000.desktop").join("context.db");
    migrate_legacy_database(&root, &database)?;
    Ok(database)
}

fn migrate_legacy_database(
    data_root: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    if destination.exists() {
        return Ok(());
    }
    let legacy = data_root.join("com.crossai.context").join("context.db");
    if legacy.exists() {
        ContextStore::open(&legacy)
            .and_then(|store| store.backup(destination))
            .map_err(|error| format!("failed to migrate the legacy TF0000 database: {error}"))?;
    }
    Ok(())
}

fn read_frame(reader: &mut impl Read) -> Result<Option<Vec<u8>>, String> {
    let mut length_bytes = [0_u8; 4];
    match reader.read_exact(&mut length_bytes) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.to_string()),
    }
    let length = u32::from_le_bytes(length_bytes) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(format!("invalid native message length: {length}"));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    Ok(Some(bytes))
}

fn write_frame(writer: &mut impl Write, message: &OutgoingMessage) -> Result<(), String> {
    let bytes = serde_json::to_vec(message).map_err(|e| e.to_string())?;
    let length = u32::try_from(bytes.len()).map_err(|_| "native message is too large")?;
    writer
        .write_all(&length.to_le_bytes())
        .and_then(|_| writer.write_all(&bytes))
        .and_then(|_| writer.flush())
        .map_err(|e| e.to_string())
}

fn run() -> Result<(), String> {
    let store = ContextStore::open(database_path()?).map_err(|e| e.to_string())?;
    let mut session = HostSession::new(store);
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    while let Some(bytes) = read_frame(&mut reader)? {
        let response = match serde_json::from_slice::<IncomingMessage>(&bytes) {
            Ok(message) => session.handle(message),
            Err(error) => failure("invalid".into(), &format!("invalid request: {error}")),
        };
        write_frame(&mut writer, &response)?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("TF0000 native host stopped: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> (tempfile::TempDir, HostSession) {
        let directory = tempfile::tempdir().expect("temp directory");
        let store = ContextStore::open(directory.path().join("context.db")).expect("store");
        (directory, HostSession::new(store))
    }

    #[test]
    fn handshake_is_required_before_requests() {
        let (_directory, mut session) = session();
        let response = session.handle(IncomingMessage::Request {
            protocol_version: 1,
            request_id: "one".into(),
            session_id: "not-authenticated".into(),
            action: "health".into(),
            payload: json!({}),
        });
        assert!(!response.ok);
    }

    #[test]
    fn handshake_echoes_nonce_and_allows_health() {
        let (_directory, mut session) = session();
        let hello = session.handle(IncomingMessage::Hello {
            protocol_version: 1,
            request_id: "one".into(),
            extension_id: "abcdefghijklmnopabcdefghijklmnop".into(),
            nonce: "0123456789abcdef".into(),
        });
        assert!(hello.ok);
        let response = session.handle(IncomingMessage::Request {
            protocol_version: 1,
            request_id: "two".into(),
            session_id: hello.session_id.expect("session id"),
            action: "health".into(),
            payload: json!({}),
        });
        assert!(response.ok);
    }

    #[test]
    fn native_frames_round_trip() {
        let message = success("request".into(), None, json!({ "ok": true }));
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &message).expect("write frame");
        let payload = read_frame(&mut bytes.as_slice())
            .expect("read frame")
            .expect("payload");
        let value: Value = serde_json::from_slice(&payload).expect("json");
        assert_eq!(value["requestId"], "request");
    }

    #[test]
    fn native_protocol_accepts_camel_case_wire_fields() {
        let message = serde_json::from_value::<IncomingMessage>(json!({
            "type": "hello",
            "protocolVersion": 1,
            "requestId": "wire-request",
            "extensionId": "abcdefghijklmnopabcdefghijklmnop",
            "nonce": "0123456789abcdef",
        }))
        .expect("camel case native message");
        assert!(matches!(message, IncomingMessage::Hello { .. }));
    }

    #[test]
    fn phase_ten_native_actions_keep_agent_writes_pending_until_confirmation() {
        let (_directory, mut session) = session();
        let project = session
            .store
            .create_project(context_core::CreateProjectInput {
                name: "VS Code project".into(),
                description: String::new(),
            })
            .expect("project");
        let hello = session.handle(IncomingMessage::Hello {
            protocol_version: 1,
            request_id: "hello".into(),
            extension_id: "abcdefghijklmnopabcdefghijklmnop".into(),
            nonce: "0123456789abcdef".into(),
        });
        let session_id = hello.session_id.expect("session id");
        let mapping = session.handle(IncomingMessage::Request {
            protocol_version: 1,
            request_id: "map".into(),
            session_id: session_id.clone(),
            action: "mapWorkspace".into(),
            payload: json!({
                "workspaceUri": "file:///E:/work/project",
                "repositoryRoot": "E:\\work\\project",
                "projectId": project.id,
            }),
        });
        assert!(mapping.ok);
        let candidate = session.handle(IncomingMessage::Request {
            protocol_version: 1,
            request_id: "candidate".into(),
            session_id: session_id.clone(),
            action: "createAgentWriteCandidate".into(),
            payload: json!({
                "projectId": project.id,
                "requestedBy": "vscode-language-model-tool",
                "memoryType": "decision",
                "title": "Use SQLite",
                "content": "Use SQLite for local state.",
            }),
        });
        assert!(candidate.ok);
        let candidate_id = candidate.data.expect("candidate")["id"]
            .as_str()
            .expect("candidate id")
            .to_string();
        let rejected = session.handle(IncomingMessage::Request {
            protocol_version: 1,
            request_id: "review".into(),
            session_id,
            action: "reviewAgentWriteCandidate".into(),
            payload: json!({
                "candidateId": candidate_id,
                "action": "accept",
                "confirmed": false,
                "memorySpaceId": null,
            }),
        });
        assert!(!rejected.ok);
    }

    #[test]
    fn phase_eleven_native_mcp_enforces_permissions_and_audits_calls() {
        let (_directory, session) = session();
        let project = session
            .store
            .create_project(context_core::CreateProjectInput {
                name: "MCP project".into(),
                description: String::new(),
            })
            .expect("project");
        session
            .store
            .register_mcp_client(RegisterMcpClientInput {
                client_id: "test-agent".into(),
                display_name: "Test Agent".into(),
            })
            .expect("MCP client");
        let read = session
            .handle_mcp_tool_call(McpToolCallPayload {
                client_id: "test-agent".into(),
                tool_name: "get_project_context".into(),
                arguments: json!({
                    "projectId": project.id,
                    "maximumCharacters": 4000,
                }),
            })
            .expect("default read access");
        assert_eq!(read["projectId"], project.id);

        let denied = session.handle_mcp_tool_call(McpToolCallPayload {
            client_id: "test-agent".into(),
            tool_name: "save_memory_candidate".into(),
            arguments: json!({
                "projectId": project.id,
                "memoryType": "decision",
                "title": "Pending only",
                "content": "This proposal must require review.",
            }),
        });
        assert!(denied.is_err());
        assert_eq!(
            session
                .store
                .agent_write_candidates(Some("pending"))
                .expect("pending candidates")
                .len(),
            0
        );
        session
            .store
            .set_mcp_permission(SetMcpPermissionInput {
                client_id: "test-agent".into(),
                project_id: Some(project.id.clone()),
                read_allowed: true,
                candidate_write_allowed: true,
            })
            .expect("allow candidate writes");
        let candidate = session
            .handle_mcp_tool_call(McpToolCallPayload {
                client_id: "test-agent".into(),
                tool_name: "save_memory_candidate".into(),
                arguments: json!({
                    "projectId": project.id,
                    "memoryType": "decision",
                    "title": "Pending only",
                    "content": "This proposal must require review.",
                }),
            })
            .expect("candidate write");
        assert_eq!(candidate["status"], "pending");
        let audit = session
            .store
            .mcp_audit_log("test-agent", 10)
            .expect("audit log");
        assert_eq!(audit.len(), 3);
        assert!(audit.iter().any(|entry| entry.outcome == "denied"));
        assert!(
            audit
                .iter()
                .any(|entry| entry.access_type == "candidate_write")
        );
    }
}
