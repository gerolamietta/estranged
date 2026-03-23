use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    str::FromStr,
};

use chrono::{DateTime, Utc};
use garde::{Unvalidated, Valid, Validate};
use langtag::LangTagBuf;
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("overflow")]
    Overflow,
    #[error(transparent)]
    Invalid(#[from] garde::Report),
    #[error("{message}")]
    Response { message: String },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct UserId(i64);

impl UserId {
    pub fn as_i64(&self) -> i64 {
        self.0
    }

    pub fn from_i64(id: i64) -> Self {
        Self(id)
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ChatId(i64);

impl ChatId {
    pub fn as_i64(&self) -> i64 {
        self.0
    }

    pub fn from_i64(id: i64) -> Self {
        Self(id)
    }
}

impl Display for ChatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mid(String);

impl Display for Mid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Seq(i64);

#[derive(Debug, Serialize, Deserialize)]
pub struct Count(i32);

impl Count {
    pub fn as_usize(&self) -> Result<usize> {
        self.0.try_into().map_err(|_| Error::Overflow)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Marker(i64);

impl Display for Marker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// #[derive(Serialize, Deserialize)]
// pub struct TimeS(#[serde(with = "chrono::serde::ts_seconds")] DateTime<Utc>);

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeMs(#[serde(with = "chrono::serde::ts_milliseconds")] DateTime<Utc>);

impl TimeMs {
    pub fn datetime(&self) -> DateTime<Utc> {
        self.0
    }
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub user_id: UserId,
    pub first_name: String,
    #[serde(default)]
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub is_bot: bool,
    pub last_activity_time: TimeMs,
}

#[derive(Deserialize)]
pub struct UserWithPhoto {
    #[serde(flatten)]
    pub user: User,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<Url>,
    #[serde(default)]
    pub full_avatar_url: Option<Url>,
}

#[derive(Debug, Serialize, Deserialize, strum::Display, strum::EnumString)]
#[serde(rename_all = "snake_case")]
pub enum ChatType {
    Chat,
    Dialog,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatStatus {
    Active,
    Removed,
    Left,
    Closed,
}

#[derive(Deserialize)]
pub struct Image {
    pub url: Url,
}

#[derive(Deserialize)]
pub struct Chat {
    pub chat_id: ChatId,
    pub r#type: ChatType,
    pub status: ChatStatus,
    pub title: Option<String>,
    pub icon: Option<Image>,
    pub last_event_time: TimeMs,
    pub participants_count: Count,
    #[serde(default)]
    pub owner_id: Option<BTreeMap<UserId, TimeMs>>,
    pub is_public: bool,
    #[serde(default)]
    pub link: Option<Url>,
    pub description: Option<String>,
    #[serde(default)]
    pub dialog_with_user: Option<UserWithPhoto>,
    #[serde(default)]
    pub chat_message_id: Option<ChatId>,
    #[serde(default)]
    pub pinned_message: Option<Message>,
}

#[derive(Debug, Deserialize)]
pub struct Recipient {
    pub chat_id: Option<ChatId>,
    pub chat_type: ChatType,
    pub user_id: Option<UserId>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageLinkType {
    Forward,
    Reply,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Attachment {
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MarkupElement {
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct MessageBody {
    pub mid: Mid,
    pub seq: Seq,
    pub text: Option<String>,
    #[serde(default)]
    pub attachments: Option<Vec<Attachment>>,
    #[serde(default)]
    pub markup: Option<Vec<MarkupElement>>,
}

#[derive(Debug, Deserialize)]
pub struct LinkedMessage {
    pub r#type: MessageLinkType,
    #[serde(default)]
    pub sender: Option<User>,
    #[serde(default)]
    pub chat_id: Option<ChatId>,
    pub message: MessageBody,
}

#[derive(Debug, Deserialize)]
pub struct MessageStat {
    pub views: Count,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub sender: Option<User>,
    pub recipient: Recipient,
    pub timestamp: TimeMs,
    #[serde(default)]
    pub link: Option<LinkedMessage>,
    pub body: MessageBody,
    #[serde(default)]
    pub stat: Option<MessageStat>,
    #[serde(default)]
    pub url: Option<Url>,
}

#[derive(Serialize, Deserialize)]
pub struct AttachmentToken(String);

#[derive(Serialize)]
pub struct AttachmentPhotos {
    token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhotoAttachmentRequestPayload {
    Url(Url),
    Token(AttachmentToken),
    Photos(AttachmentPhotos),
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AttachmentRequest {
    Image {
        payload: PhotoAttachmentRequestPayload,
    },
}

#[derive(Serialize)]
pub struct NewMessageLink {
    pub r#type: MessageLinkType,
    pub mid: Mid,
}

#[derive(Serialize)]
pub struct Notify(bool);

impl Default for Notify {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextFormat {
    Markdown,
    Html,
}

#[derive(Serialize, Default)]
pub struct NewMessageBody {
    pub text: Option<String>,
    pub attachments: Option<Vec<AttachmentRequest>>,
    pub link: Option<NewMessageLink>,
    #[serde(default)]
    pub notify: Notify,
    #[serde(default)]
    pub format: Option<TextFormat>,
}

#[derive(Debug, Deserialize, EnumDiscriminants)]
#[serde(rename_all = "snake_case", tag = "update_type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
#[strum_discriminants(name(UpdateType))]
#[strum_discriminants(derive(
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    PartialOrd,
    Ord,
))]
pub enum UpdateKind {
    MessageCreated {
        message: Box<Message>,
        #[serde(default)]
        user_locale: Option<LangTagBuf>,
    },
    BotStarted {
        chat_id: ChatId,
        user: Box<User>,
        #[serde(default)]
        payload: Option<String>,
        #[serde(default)]
        user_locale: Option<LangTagBuf>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct Update {
    pub timestamp: TimeMs,
    #[serde(flatten)]
    pub kind: UpdateKind,
}

#[derive(Serialize, Deserialize)]
pub struct Subscription {
    pub url: Url,
    pub time: TimeMs,
    pub update_types: Option<BTreeSet<UpdateType>>,
}

#[derive(Serialize, Deserialize, Validate, Clone, PartialEq, Eq)]
struct SecretInner(#[garde(pattern("^[a-zA-Z0-9_-]{5,256}$"))] String);

#[derive(Clone)]
struct ValidSerde<T>(Valid<T>);

impl<'de, T: Deserialize<'de> + Validate<Context: Default>> Deserialize<'de> for ValidSerde<T> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Unvalidated::deserialize(deserializer)?
            .validate()
            .map_err(serde::de::Error::custom)
            .map(Self)
    }
}

impl<T: Serialize> Serialize for ValidSerde<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (*self.0).serialize(serializer)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Secret(ValidSerde<SecretInner>);

impl PartialEq for Secret {
    fn eq(&self, other: &Self) -> bool {
        *self.0.0 == *other.0.0
    }
}

impl Eq for Secret {}

impl FromStr for Secret {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(Self(ValidSerde(
            Unvalidated::new(SecretInner(s.into())).validate()?,
        )))
    }
}

impl Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (*self.0.0).0.fmt(f)
    }
}

#[derive(Serialize)]
pub struct SubscriptionRequest {
    pub url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_types: Option<BTreeSet<UpdateType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<Secret>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum RequestResult {
    Ok {
        success: MustBe!(true),
    },
    Err {
        success: MustBe!(false),
        message: String,
    },
}

impl RequestResult {
    pub fn into_result(self) -> Result<()> {
        match self {
            Self::Ok { .. } => Ok(()),
            Self::Err { message, .. } => Err(Error::Response { message }),
        }
    }
}

#[derive(Deserialize)]
pub struct Updates {
    pub updates: Vec<Update>,
    pub marker: Option<Marker>,
}

#[derive(Deserialize)]
pub struct SendResult {
    pub message: Message,
}
