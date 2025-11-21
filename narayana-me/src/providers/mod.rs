//! Avatar provider implementations

#[cfg(feature = "beyond-presence")]
pub mod beyond_presence;

#[cfg(feature = "beyond-presence")]
pub use beyond_presence::BeyondPresenceProvider;

pub mod live_avatar;
pub use live_avatar::LiveAvatarProvider;

pub mod ready_player_me;
pub use ready_player_me::ReadyPlayerMeProvider;

pub mod avatar_sdk;
pub use avatar_sdk::AvatarSDKProvider;

pub mod open_avatar_chat;
pub use open_avatar_chat::OpenAvatarChatProvider;

