//! Enumerate the channel types compiled into this binary.
//!
//! Use [`compiled_channels`] in display commands (`zeroclaw channel list`) that
//! should only mention channels that can actually be started.  For a full
//! channel inventory regardless of compile-time features, use
//! [`zeroclaw_config::schema::ChannelsConfig::channels`] instead.

use zeroclaw_config::schema::ChannelsConfig;
use zeroclaw_config::traits::ChannelInfo;

/// Returns one entry per channel type compiled into this binary.
///
/// Filters the canonical channel list from [`ChannelsConfig::channels`] down to
/// only those enabled at compile time via `channel-*` / `voice-wake` feature
/// flags. Name, desc, and configured status come from the config crate's single
/// source of truth; this function contributes only the compile-time filter.
pub fn compiled_channels(cfg: &ChannelsConfig) -> Vec<ChannelInfo> {
    // Each entry is the display name of one channel, included only when its
    // corresponding feature flag is enabled at compile time.
    let compiled: &[&str] = &[
        #[cfg(feature = "channel-telegram")]
        "Telegram",
        #[cfg(feature = "channel-discord")]
        "Discord",
        #[cfg(feature = "channel-slack")]
        "Slack",
        #[cfg(feature = "channel-mattermost")]
        "Mattermost",
        #[cfg(feature = "channel-imessage")]
        "iMessage",
        #[cfg(feature = "channel-matrix")]
        "Matrix",
        #[cfg(feature = "channel-signal")]
        "Signal",
        #[cfg(feature = "channel-whatsapp-cloud")]
        "WhatsApp",
        #[cfg(feature = "whatsapp-web")]
        "WhatsApp Web",
        #[cfg(feature = "channel-linq")]
        "Linq",
        #[cfg(feature = "channel-wati")]
        "WATI",
        #[cfg(feature = "channel-nextcloud")]
        "NextCloud Talk",
        #[cfg(feature = "channel-email")]
        "Email",
        #[cfg(feature = "channel-email")]
        "Gmail Push",
        #[cfg(feature = "channel-irc")]
        "IRC",
        #[cfg(feature = "channel-lark")]
        "Lark",
        #[cfg(feature = "channel-dingtalk")]
        "DingTalk",
        #[cfg(feature = "channel-wecom")]
        "WeCom",
        #[cfg(feature = "channel-wecom-ws")]
        "WeCom WebSocket",
        #[cfg(feature = "channel-wechat")]
        "WeChat",
        #[cfg(feature = "channel-qq")]
        "QQ Official",
        #[cfg(feature = "channel-nostr")]
        "Nostr",
        #[cfg(feature = "channel-clawdtalk")]
        "ClawdTalk",
        #[cfg(feature = "channel-reddit")]
        "Reddit",
        #[cfg(feature = "channel-bluesky")]
        "Bluesky",
        #[cfg(feature = "channel-twitter")]
        "X/Twitter",
        #[cfg(feature = "channel-mochat")]
        "Mochat",
        #[cfg(feature = "channel-line")]
        "LINE",
        #[cfg(feature = "channel-voice-call")]
        "Voice Call",
        #[cfg(feature = "voice-wake")]
        "VoiceWake",
        #[cfg(feature = "channel-mqtt")]
        "MQTT",
        #[cfg(feature = "channel-webhook")]
        "Webhook",
    ];

    cfg.channels()
        .into_iter()
        .filter(|info| compiled.contains(&info.name))
        .collect()
}

/// Returns whether a schema channel type key is compiled into this binary.
///
/// Accepts both kebab-case keys emitted by the config schema and legacy
/// underscore spellings used in channel references.
pub fn is_channel_type_compiled(channel_type: &str) -> bool {
    match channel_type {
        "telegram" => cfg!(feature = "channel-telegram"),
        "discord" => cfg!(feature = "channel-discord"),
        "slack" => cfg!(feature = "channel-slack"),
        "mattermost" => cfg!(feature = "channel-mattermost"),
        "imessage" => cfg!(feature = "channel-imessage"),
        "matrix" => cfg!(feature = "channel-matrix"),
        "signal" => cfg!(feature = "channel-signal"),
        "whatsapp" => cfg!(feature = "channel-whatsapp-cloud"),
        "whatsapp-web" | "whatsapp_web" => cfg!(feature = "whatsapp-web"),
        "linq" => cfg!(feature = "channel-linq"),
        "wati" => cfg!(feature = "channel-wati"),
        "nextcloud-talk" | "nextcloud_talk" => cfg!(feature = "channel-nextcloud"),
        "email" | "gmail-push" | "gmail_push" => cfg!(feature = "channel-email"),
        "irc" => cfg!(feature = "channel-irc"),
        "lark" | "feishu" => cfg!(feature = "channel-lark"),
        "dingtalk" => cfg!(feature = "channel-dingtalk"),
        "wecom" => cfg!(feature = "channel-wecom"),
        "wecom-ws" | "wecom_ws" => cfg!(feature = "channel-wecom-ws"),
        "wechat" => cfg!(feature = "channel-wechat"),
        "qq" => cfg!(feature = "channel-qq"),
        "nostr" => cfg!(feature = "channel-nostr"),
        "clawdtalk" => cfg!(feature = "channel-clawdtalk"),
        "reddit" => cfg!(feature = "channel-reddit"),
        "bluesky" => cfg!(feature = "channel-bluesky"),
        "twitter" => cfg!(feature = "channel-twitter"),
        "mochat" => cfg!(feature = "channel-mochat"),
        "line" => cfg!(feature = "channel-line"),
        "voice-call" | "voice_call" => cfg!(feature = "channel-voice-call"),
        "voice-wake" | "voice_wake" => cfg!(feature = "voice-wake"),
        "mqtt" => cfg!(feature = "channel-mqtt"),
        "webhook" => cfg!(feature = "channel-webhook"),
        "acp-server" | "acp_server" => cfg!(feature = "channel-acp-server"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_channel_type_compiled;

    #[cfg(feature = "default-channels")]
    #[test]
    fn channel_type_compilation_matches_default_bundle() {
        assert!(is_channel_type_compiled("telegram"));
        assert!(is_channel_type_compiled("email"));
        assert!(is_channel_type_compiled("webhook"));
        assert!(is_channel_type_compiled("acp-server"));
        assert!(!is_channel_type_compiled("nextcloud-talk"));
        assert!(!is_channel_type_compiled("linq"));
    }
}
