//! A2UI v1.0 envelope types and the builders that turn a surface draft into
//! protocol messages.
//!
//! The server never emits `deleteSurface`, so there is no builder for it.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::catalog::CATALOG_ID;

const PROTOCOL_VERSION: &str = "1.0";

fn default_true() -> bool {
    true
}

// -- Envelope --

/// The `"version": "1.0"` marker; any other value fails to deserialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProtocolVersion;

impl Serialize for ProtocolVersion {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(PROTOCOL_VERSION)
    }
}

impl<'de> Deserialize<'de> for ProtocolVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = String::deserialize(d)?;
        if v == PROTOCOL_VERSION {
            Ok(ProtocolVersion)
        } else {
            Err(D::Error::custom(format!(
                "unsupported A2UI version \"{v}\", expected \"{PROTOCOL_VERSION}\""
            )))
        }
    }
}

/// One agent-to-renderer message: a version plus exactly one body key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub version: ProtocolVersion,
    #[serde(flatten)]
    pub body: MessageBody,
}

impl Message {
    fn new(body: MessageBody) -> Self {
        Self {
            version: ProtocolVersion,
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageBody {
    CreateSurface(CreateSurface),
    UpdateComponents(UpdateComponents),
    UpdateDataModel(UpdateDataModel),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSurface {
    pub surface_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_data_model: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_model: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateComponents {
    pub surface_id: String,
    pub components: Vec<Component>,
}

/// `value: null` deletes the key at `path`; an absent `path` means `/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDataModel {
    pub surface_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub value: Value,
}

// -- Components --

/// One entry of the flat component list: an id, a catalog type and its
/// type-specific properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub component: String,
    #[serde(flatten)]
    pub props: Map<String, Value>,
}

impl Component {
    /// The component as one JSON object, as the catalog schemas expect it.
    pub fn to_value(&self) -> Value {
        let mut obj = self.props.clone();
        obj.insert("id".to_owned(), Value::String(self.id.clone()));
        obj.insert(
            "component".to_owned(),
            Value::String(self.component.clone()),
        );
        Value::Object(obj)
    }
}

// -- Drafts and builders --

/// A complete surface as the model describes it on create.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceDraft {
    pub title: String,
    pub fallback_text: String,
    pub components: Vec<Component>,
    #[serde(default)]
    pub data_model: Option<Value>,
    #[serde(default = "default_true")]
    pub send_data_model: bool,
}

/// One `{path, value}` entry of an update; `null` deletes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataUpdate {
    pub path: String,
    pub value: Value,
}

/// An incremental change to an existing surface.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SurfacePatch {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub fallback_text: Option<String>,
    #[serde(default)]
    pub components: Vec<Component>,
    #[serde(default)]
    pub data_updates: Vec<DataUpdate>,
}

/// Builders for the messages the server emits.
pub struct SurfaceMessages;

impl SurfaceMessages {
    /// The `createSurface` message for a new surface.
    pub fn create(draft: &SurfaceDraft, surface_id: &str) -> Message {
        Message::new(MessageBody::CreateSurface(CreateSurface {
            surface_id: surface_id.to_owned(),
            catalog_id: Some(CATALOG_ID.to_owned()),
            send_data_model: Some(draft.send_data_model),
            components: draft.components.clone(),
            data_model: draft.data_model.clone(),
        }))
    }

    /// The messages for a patch: `updateComponents` when components changed,
    /// then one `updateDataModel` per data update.
    pub fn update(surface_id: &str, patch: &SurfacePatch) -> Vec<Message> {
        let components = (!patch.components.is_empty()).then(|| {
            Message::new(MessageBody::UpdateComponents(UpdateComponents {
                surface_id: surface_id.to_owned(),
                components: patch.components.clone(),
            }))
        });
        let data = patch.data_updates.iter().map(|u| {
            Message::new(MessageBody::UpdateDataModel(UpdateDataModel {
                surface_id: surface_id.to_owned(),
                path: Some(u.path.clone()),
                value: u.value.clone(),
            }))
        });
        components.into_iter().chain(data).collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::CATALOG_ID;

    fn roundtrip(v: Value) -> Message {
        let m: Message = serde_json::from_value(v.clone()).expect("deserialize");
        assert_eq!(serde_json::to_value(&m).expect("serialize"), v);
        m
    }

    #[test]
    fn create_surface_example_round_trips() {
        let m = roundtrip(json!({
            "version": "1.0",
            "createSurface": {
                "surfaceId": "user_profile_card",
                "catalogId": "https://a2ui.org/specification/v1_0/catalogs/basic/catalog.json",
                "sendDataModel": true,
                "components": [
                    {"id": "root", "component": "Column", "children": ["user_name"]},
                    {"id": "user_name", "component": "Text", "text": {"@path": "/name"}}
                ],
                "dataModel": {"name": "John Doe"}
            }
        }));
        let MessageBody::CreateSurface(c) = m.body else {
            panic!("expected createSurface");
        };
        assert_eq!(c.surface_id, "user_profile_card");
        assert_eq!(c.components.len(), 2);
        assert_eq!(c.components[1].id, "user_name");
        assert_eq!(c.components[1].component, "Text");
    }

    #[test]
    fn update_components_example_round_trips() {
        let m = roundtrip(json!({
            "version": "1.0",
            "updateComponents": {
                "surfaceId": "user_profile_card",
                "components": [
                    {"id": "root", "component": "Column", "children": ["user_name", "user_title"]},
                    {"id": "user_name", "component": "Text", "text": "John Doe"},
                    {"id": "user_title", "component": "Text", "text": "Software Engineer"}
                ]
            }
        }));
        assert!(matches!(m.body, MessageBody::UpdateComponents(_)));
    }

    #[test]
    fn update_data_model_example_round_trips() {
        let m = roundtrip(json!({
            "version": "1.0",
            "updateDataModel": {
                "surfaceId": "user_profile_card",
                "path": "/user/name",
                "value": "Jane Doe"
            }
        }));
        assert!(matches!(m.body, MessageBody::UpdateDataModel(_)));
    }

    #[test]
    fn update_data_model_keeps_explicit_null() {
        roundtrip(json!({
            "version": "1.0",
            "updateDataModel": {"surfaceId": "s", "path": "/a", "value": null}
        }));
    }

    #[test]
    fn other_protocol_versions_are_rejected() {
        let v = json!({
            "version": "0.9",
            "updateDataModel": {"surfaceId": "s", "path": "/a", "value": 1}
        });
        assert!(serde_json::from_value::<Message>(v).is_err());
    }

    fn draft() -> SurfaceDraft {
        serde_json::from_value(json!({
            "title": "Profile",
            "fallback_text": "Profile of John",
            "components": [
                {"id": "root", "component": "Text", "text": {"@path": "/name"}}
            ],
            "data_model": {"name": "John"}
        }))
        .expect("draft")
    }

    #[test]
    fn draft_defaults_send_data_model_to_true() {
        assert!(draft().send_data_model);
    }

    #[test]
    fn create_builds_a_create_surface_with_catalog_id() {
        let v = serde_json::to_value(SurfaceMessages::create(&draft(), "s1")).expect("serialize");
        assert_eq!(v["version"], "1.0");
        assert_eq!(v["createSurface"]["surfaceId"], "s1");
        assert_eq!(v["createSurface"]["catalogId"], CATALOG_ID);
        assert_eq!(v["createSurface"]["sendDataModel"], true);
        assert_eq!(v["createSurface"]["dataModel"], json!({"name": "John"}));
        assert_eq!(v["createSurface"]["components"][0]["id"], "root");
    }

    #[test]
    fn create_omits_absent_data_model() {
        let mut d = draft();
        d.data_model = None;
        d.send_data_model = false;
        let v = serde_json::to_value(SurfaceMessages::create(&d, "s1")).expect("serialize");
        assert!(v["createSurface"].get("dataModel").is_none());
        assert_eq!(v["createSurface"]["sendDataModel"], false);
    }

    fn patch(v: Value) -> SurfacePatch {
        serde_json::from_value(v).expect("patch")
    }

    #[test]
    fn update_emits_components_then_one_data_message_per_entry() {
        let p = patch(json!({
            "components": [{"id": "a", "component": "Text", "text": "x"}],
            "data_updates": [
                {"path": "/a", "value": 1},
                {"path": "/b", "value": null}
            ]
        }));
        let msgs: Vec<Value> = SurfaceMessages::update("s1", &p)
            .iter()
            .map(|m| serde_json::to_value(m).expect("serialize"))
            .collect();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0]["updateComponents"]["surfaceId"], "s1");
        assert_eq!(msgs[1]["updateDataModel"]["path"], "/a");
        assert_eq!(msgs[2]["updateDataModel"]["value"], Value::Null);
    }

    #[test]
    fn update_without_components_emits_no_update_components() {
        let p = patch(json!({"data_updates": [{"path": "/a", "value": "x"}]}));
        let msgs = SurfaceMessages::update("s1", &p);
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0].body, MessageBody::UpdateDataModel(_)));
    }
}
