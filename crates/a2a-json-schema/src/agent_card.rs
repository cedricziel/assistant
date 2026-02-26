//! Agent discovery manifest types.
//!
//! The `AgentCard` is a self-describing manifest for an agent, providing
//! metadata including identity, capabilities, skills, supported communication
//! methods, and security requirements.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::security::{SecurityRequirement, SecurityScheme};

/// A self-describing manifest for an agent.
///
/// It provides essential metadata including the agent's identity, capabilities,
/// skills, supported communication methods, and security requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    /// A human-readable name for the agent.
    pub name: String,

    /// A human-readable description of the agent's purpose.
    pub description: String,

    /// Ordered list of supported interfaces. The first entry is preferred.
    pub supported_interfaces: Vec<AgentInterface>,

    /// The service provider of the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<AgentProvider>,

    /// The version of the agent (e.g., "1.0.0").
    pub version: String,

    /// A URL providing additional documentation about the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,

    /// A2A capability set supported by the agent.
    pub capabilities: AgentCapabilities,

    /// Security scheme definitions for authenticating with this agent.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub security_schemes: HashMap<String, SecurityScheme>,

    /// Security requirements for contacting the agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security_requirements: Vec<SecurityRequirement>,

    /// Input modes the agent supports across all skills (media types).
    pub default_input_modes: Vec<String>,

    /// Output media types supported by this agent.
    pub default_output_modes: Vec<String>,

    /// Skills represent the abilities of an agent.
    pub skills: Vec<AgentSkill>,

    /// JSON Web Signatures computed for this `AgentCard`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signatures: Vec<AgentCardSignature>,

    /// A URL to an icon for the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

/// Declares a combination of a target URL, transport, and protocol version for
/// interacting with the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInterface {
    /// The URL where this interface is available.
    pub url: String,

    /// The protocol binding (e.g., "JSONRPC", "GRPC", "HTTP+JSON").
    pub protocol_binding: String,

    /// Tenant ID to be used in the request when calling the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// The version of the A2A protocol this interface exposes (e.g., "1.0").
    pub protocol_version: String,
}

/// Represents the service provider of an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvider {
    /// A URL for the provider's website or documentation.
    pub url: String,

    /// The name of the provider's organization.
    pub organization: String,
}

/// Defines optional capabilities supported by an agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    /// Indicates if the agent supports streaming responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,

    /// Indicates if the agent supports push notifications.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub push_notifications: Option<bool>,

    /// Protocol extensions supported by the agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<AgentExtension>,

    /// Indicates if the agent supports providing an extended agent card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_agent_card: Option<bool>,
}

/// A declaration of a protocol extension supported by an Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExtension {
    /// The unique URI identifying the extension.
    pub uri: String,

    /// A human-readable description of how this agent uses the extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// If true, the client must understand and comply with the extension.
    #[serde(default)]
    pub required: bool,

    /// Extension-specific configuration parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// Represents a distinct capability or function that an agent can perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    /// A unique identifier for the skill.
    pub id: String,

    /// A human-readable name for the skill.
    pub name: String,

    /// A detailed description of the skill.
    pub description: String,

    /// Keywords describing the skill's capabilities.
    pub tags: Vec<String>,

    /// Example prompts or scenarios that this skill can handle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,

    /// Supported input media types, overriding agent defaults.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_modes: Vec<String>,

    /// Supported output media types, overriding agent defaults.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_modes: Vec<String>,

    /// Security schemes necessary for this skill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security_requirements: Vec<SecurityRequirement>,
}

/// Represents a JWS signature of an AgentCard (RFC 7515).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCardSignature {
    /// The protected JWS header, base64url-encoded JSON object.
    #[serde(rename = "protected")]
    pub protected_header: String,

    /// The computed signature, base64url-encoded.
    pub signature: String,

    /// The unprotected JWS header values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<HashMap<String, serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_card_roundtrip() {
        let card = AgentCard {
            name: "Test Agent".to_string(),
            description: "An agent for testing".to_string(),
            supported_interfaces: vec![AgentInterface {
                url: "https://example.com/a2a".to_string(),
                protocol_binding: "HTTP+JSON".to_string(),
                tenant: None,
                protocol_version: "1.0".to_string(),
            }],
            provider: Some(AgentProvider {
                url: "https://example.com".to_string(),
                organization: "Test Org".to_string(),
            }),
            version: "1.0.0".to_string(),
            documentation_url: None,
            capabilities: AgentCapabilities {
                streaming: Some(true),
                push_notifications: Some(false),
                extensions: vec![],
                extended_agent_card: None,
            },
            security_schemes: HashMap::new(),
            security_requirements: vec![],
            default_input_modes: vec!["text/plain".to_string()],
            default_output_modes: vec!["text/plain".to_string()],
            skills: vec![AgentSkill {
                id: "general".to_string(),
                name: "General Assistant".to_string(),
                description: "General-purpose assistance".to_string(),
                tags: vec!["general".to_string()],
                examples: vec!["Help me with a task".to_string()],
                input_modes: vec![],
                output_modes: vec![],
                security_requirements: vec![],
            }],
            signatures: vec![],
            icon_url: None,
        };

        let json = serde_json::to_string_pretty(&card).unwrap();
        let deserialized: AgentCard = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test Agent");
        assert_eq!(deserialized.skills.len(), 1);
        assert_eq!(deserialized.capabilities.streaming, Some(true));
    }
}
