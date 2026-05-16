pub mod allowlist;
pub mod attachment;
pub mod auth;
pub mod bus;
pub mod bus_messages;
pub mod catalog;
pub mod channel;
pub mod command;
pub mod config;
pub mod context;
pub mod identity;
pub mod llm;
pub mod memory;
pub mod store;
pub mod subagent;
pub mod text;
pub mod tool;
pub mod types;
pub mod upload;

pub use allowlist::AllowlistFilter;
pub use attachment::{
    AttachmentMeta, MAX_ATTACHMENT_SIZE, RESIZABLE_MIME_TYPES, SUPPORTED_MIME_TYPES,
    extension_for_mime, is_resizable_mime_type, is_supported_mime_type, is_text_mime_type,
};
// -- Auth & identity (multi-user) --
pub use auth::{
    ApiKeyRecord, ApiKeyStore, AuthCode, AuthCodeStore, AuthContext, AuthProvider,
    AuthorizeRequest, AuthorizeResponse, ClientInfo, ClientRegistration, ClientStore,
    DeviceCodeStore, DeviceState, IdentityResolver, RefreshTokenStore, StoredRefreshToken,
    TokenGrant, TokenResponse,
};
pub use bus::{
    BusMessage, ClaimFilter, MAX_TURN_REDELIVERIES, MessageBus, MessageStatus, PublishRequest,
};
pub use bus_messages::{
    AgentReport, AgentReportStatus, AgentSpawn, ToolExecute, ToolResult, TurnPhase, TurnRequest,
    TurnResult, TurnStatus, topic,
};
pub use catalog::{CatalogEntry, CatalogResolver, CatalogResourceType};
pub use channel::{ChannelAdapter, ChannelContent, ChannelMessage, ChannelUser};
pub use command::{CommandArg, CommandDef, CommandResult, ConversationConfig};
pub use config::{default_config_path, load_config};
pub use context::{
    agent_base_dir, apply_agent_context, default_agent_id, default_workspace_dir,
    runtime_agent_root, runtime_workspace_dir, set_runtime_agent_root, set_runtime_workspace_dir,
    validate_agent_id,
};
pub use identity::{Action, OrgId, ResourceKind, Role, Scope, SpaceId, UserId};
// -- Multi-tenant stores --
pub use store::{
    BindingStore, CatalogItem, CatalogItemStore, CatalogSubscription, CatalogSubscriptionStore,
    InMemoryBindingStore, InMemoryCatalogItemStore, InMemoryCatalogSubscriptionStore,
    InMemoryInterfaceInstanceStore, InMemoryMembershipStore, InMemoryOrgStore, InMemorySpaceStore,
    InMemoryUserStore, InterfaceInstance, InterfaceInstanceStore, MembershipStore, OrgStore,
    Organization, PersonaBinding, Space, SpaceMembership, SpaceStore, User, UserStore,
};
// -- LLM abstractions (types, traits, streaming) --
pub use llm::embedding::{EmbeddingProvider, LlmEmbedder, WithEmbeddingOverride};
pub use llm::provider::{Capabilities, HostedTool, LlmProvider, ToolSupport};
pub use llm::stream_chunk::StreamChunk;
pub use llm::tool_spec::ToolSpec;
pub use llm::types::{
    ChatHistoryMessage, ChatRole, ContentBlock, LlmResponse, LlmResponseMeta, ToolCallItem,
    ToolCallResponse,
};
pub use memory::{
    MemoryLoader, base_dir, expand_tilde, resolve_dir, resolve_path, strip_html_comments,
};
pub use subagent::SubagentRunner;
pub use text::{preview, sanitize_llm_output, strip_cite_tags, strip_think_tags};
pub use tool::{Attachment, ToolHandler, ToolOutput};
// `assistant_core::types::observability::OtelExporter`) rather than from the
// crate root.
pub use types::{
    AgentConfig, AssistantConfig, ChannelType, DEFAULT_MAX_AGENT_DEPTH, ExecutionContext,
    Interface, Message, MessageRole, TurnIdentity,
};
pub use upload::resolve_upload_bytes;
