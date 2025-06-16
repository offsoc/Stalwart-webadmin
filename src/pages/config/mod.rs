/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

pub mod edit;
pub mod list;
pub mod schema;
pub mod search;

use std::{collections::BTreeMap, str::FromStr};

use crate::{
    components::{
        form::input::{Duration, Rate},
        icon::{
            IconAdjustmentsHorizontal, IconBeaker, IconChartBarSquare, IconCircleStack,
            IconCodeBracket, IconHandRaised, IconInbox, IconInboxArrowDown, IconInboxStack,
            IconKey, IconServer, IconServerStack, IconShieldCheck, IconSignal,
        },
        layout::{LayoutBuilder, MenuItem, LayoutConfig},
    },
    core::{
        form::{FormData, FormValue},
        schema::*,
    },
};
use ahash::AHashMap;
use humansize::{format_size, DECIMAL};
use leptos::view;
use serde::{Deserialize, Serialize};

pub type Settings = AHashMap<String, String>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum UpdateSettings {
    Delete {
        keys: Vec<String>,
    },
    Clear {
        prefix: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        filter: Option<String>,
    },
    Insert {
        prefix: Option<String>,
        values: Vec<(String, String)>,
        assert_empty: bool,
    },
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct ReloadSettings {
    pub warnings: BTreeMap<String, ConfigWarning>,
    pub errors: BTreeMap<String, ConfigError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum ConfigWarning {
    Missing,
    AppliedDefault { default: String },
    Unread { value: String },
    Build { error: String },
    Parse { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum ConfigError {
    Parse { error: String },
    Build { error: String },
    Macro { error: String },
}

impl FormData {
    pub fn build_update(&self) -> Vec<UpdateSettings> {
        let mut updates = Vec::new();
        let mut insert_prefix = None;
        let mut assert_empty = false;

        match &self.schema.typ {
            SchemaType::Record { prefix, .. } => {
                if self.is_update {
                    updates.push(UpdateSettings::Clear {
                        prefix: format!("{prefix}.{}.", self.value_as_str("_id").unwrap()),
                        filter: None,
                    });
                } else {
                    assert_empty = true;
                }

                insert_prefix = format!("{prefix}.{}", self.value_as_str("_id").unwrap()).into();
            }
            SchemaType::Entry { prefix } => {
                updates.push(UpdateSettings::Insert {
                    prefix: None,
                    assert_empty: !self.is_update,
                    values: vec![(
                        format!("{prefix}.{}", self.value_as_str("_id").unwrap()),
                        self.value_as_str("_value").unwrap_or_default().to_string(),
                    )],
                });
                return updates;
            }
            SchemaType::List => {
                if self.is_update {
                    let mut delete_keys = Vec::new();
                    for field in self.schema.fields.values() {
                        if field.is_multivalue() {
                            updates.push(UpdateSettings::Clear {
                                prefix: format!("{}.", field.id),
                                filter: None,
                            });
                            delete_keys.push(field.id.to_string());
                        } else if self.value_is_empty(field.id) {
                            delete_keys.push(field.id.to_string());
                        }
                    }

                    if !delete_keys.is_empty() {
                        updates.push(UpdateSettings::Delete { keys: delete_keys });
                    }
                }
            }
        }

        let mut key_values = Vec::new();
        for (key, value) in &self.values {
            if key.starts_with('_') {
                continue;
            }

            match value {
                FormValue::Value(value) if !value.is_empty() => {
                    key_values.push((key.to_string(), value.to_string()));
                }
                FormValue::Array(values) if !values.is_empty() => {
                    let total_values = values.len();
                    if total_values > 1 {
                        let pad_len = (total_values - 1).to_string().len();

                        for (idx, value) in values.iter().enumerate() {
                            key_values.push((format!("{key}.{idx:0>pad_len$}"), value.to_string()));
                        }
                    } else {
                        key_values.push((key.to_string(), values.first().unwrap().to_string()));
                    }
                }
                FormValue::Expression(expr) if !expr.is_empty() => {
                    if !expr.if_thens.is_empty() {
                        let total_values = expr.if_thens.len();
                        let pad_len = total_values.to_string().len();

                        for (idx, if_then) in expr.if_thens.iter().enumerate() {
                            key_values.push((
                                format!("{key}.{idx:0>pad_len$}.if"),
                                if_then.if_.to_string(),
                            ));
                            key_values.push((
                                format!("{key}.{idx:0>pad_len$}.then"),
                                if_then.then_.to_string(),
                            ));
                        }

                        key_values.push((
                            format!("{key}.{total_values:0>pad_len$}.else"),
                            expr.else_.to_string(),
                        ));
                    } else {
                        key_values.push((key.to_string(), expr.else_.to_string()));
                    }
                }
                _ => (),
            }
        }

        if !key_values.is_empty() {
            updates.push(UpdateSettings::Insert {
                prefix: insert_prefix,
                values: key_values,
                assert_empty,
            });
        }

        updates
    }
}

pub trait SettingsValues {
    fn array_values(&self, prefix: &str) -> Vec<(&str, &str)>;
    fn format(&self, field: &Field) -> String;
}

impl SettingsValues for Settings {
    fn array_values(&self, key: &str) -> Vec<(&str, &str)> {
        let full_prefix = key;
        let prefix = format!("{key}.");

        let mut results = self
            .iter()
            .filter_map(move |(key, value)| {
                if key.starts_with(&prefix) || key == full_prefix {
                    (key.as_str(), value.as_str()).into()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Sort by key
        results.sort_by(|(l_key, _), (r_key, _)| l_key.cmp(r_key));
        results
    }

    fn format(&self, field: &Field) -> String {
        match &field.typ_ {
            Type::Select {
                source: Source::Static(items),
                typ: SelectType::Single,
            } => {
                let value = self
                    .get(field.id)
                    .map(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string();
                items
                    .iter()
                    .find_map(|(k, v)| if k == &value { Some(*v) } else { None })
                    .map(|s| s.to_string())
                    .unwrap_or(value)
            }
            Type::Array => self
                .array_values(field.id)
                .first()
                .map(|(_, v)| v.to_string())
                .unwrap_or_default(),
            Type::Boolean => {
                if self.get(field.id).is_some_and(|s| s == "true") {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                }
            }
            Type::Duration => self
                .get(field.id)
                .and_then(|s| Duration::from_str(s).ok())
                .and_then(|d| d.format())
                .unwrap_or_default(),
            Type::Rate => self
                .get(field.id)
                .and_then(|s| Rate::from_str(s).ok())
                .and_then(|d| d.format())
                .unwrap_or_default(),
            Type::Size => self
                .get(field.id)
                .and_then(|s| s.parse::<u64>().ok())
                .map(|s| format_size(s, DECIMAL))
                .unwrap_or_default(),
            _ => self
                .get(field.id)
                .map(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
        }
    }
}

impl LayoutBuilder {
    pub fn settings(manage_url: &'static str) -> Vec<MenuItem> {
        LayoutBuilder::new("/settings")
            // Server
            .create("Server")
            .icon(view! { <IconServerStack/> })
            // Network
            .create("Network")
            .route("/network/edit")
            .insert(true)
            // System
            .create("System")
            .route("/system/edit")
            .insert(true)
            // Listener
            .create("Listeners")
            .route("/listener")
            .insert(true)
            // TLS
            .create("TLS")
            .create("ACME Providers")
            .route("/acme")
            .insert(true)
            .create("Certificates")
            .route("/certificate")
            .insert(true)
            .create("Defaults")
            .route("/tls/edit")
            .insert(true)
            .insert(true)
            // System
            .create("Cluster")
            .route("/cluster/edit")
            .insert(true)
            // Cache
            .create("Cache")
            .route("/cache/edit")
            .insert(true)
            // Enterprise
            .create("AI Models")
            .route("/ai-models")
            .insert(true)
            // Enterprise
            .create("Enterprise")
            .route("/enterprise/edit")
            .insert(true)
            .insert(true)
            // Storage
            .create("Storage")
            .icon(view! { <IconCircleStack/> })
            .create("Settings")
            .route("/storage/edit")
            .insert(true)
            .create("Stores")
            .route("/store")
            .insert(true)
            .create("HTTP lists")
            .route("/http-lookup")
            .insert(true)
            .insert(true)
            // Authentication
            .create("Authentication")
            .icon(view! { <IconKey/> })
            .create("Settings")
            .route("/authentication/edit")
            .insert(true)
            .create("Directories")
            .route("/directory")
            .insert(true)
            .create("OAuth")
            .route("/oauth/edit")
            .insert(true)
            .create("OpenID Connect")
            .route("/openid/edit")
            .insert(true)
            .insert(true)
            // HTTP
            .create("HTTP")
            .icon(view! { <IconInboxStack/> })
            .create("JMAP")
            .create("Push Notifications")
            .route("/jmap-push/edit")
            .insert(true)
            .create("Web Sockets")
            .route("/jmap-web-sockets/edit")
            .insert(true)
            .create("Protocol Limits")
            .route("/jmap-limits/edit")
            .insert(true)
            .insert(true)
            .create("WebDAV")
            .create("Settings")
            .route("/webdav/edit")
            .insert(true)
            .create("CalDAV")
            .route("/caldav/edit")
            .insert(true)
            .create("CardDAV")
            .route("/carddav/edit")
            .insert(true)
            .insert(true)
            .create("Settings")
            .route("/http-settings/edit")
            .insert(true)
            .create("Security")
            .route("/http-security/edit")
            .insert(true)
            .create("Rate Limits")
            .route("/http-rate-limit/edit")
            .insert(true)
            .create("Form submission")
            .route("/http-form/edit")
            .insert(true)
            .insert(true)
            // SMTP
            .create("SMTP")
            .icon(view! { <IconInboxArrowDown/> })
            .create("Inbound")
            .create("Connect stage")
            .route("/smtp-in-connect/edit")
            .insert(true)
            .create("EHLO stage")
            .route("/smtp-in-ehlo/edit")
            .insert(true)
            .create("AUTH stage")
            .route("/smtp-in-auth/edit")
            .insert(true)
            .create("MAIL stage")
            .route("/smtp-in-mail/edit")
            .insert(true)
            .create("RCPT stage")
            .route("/smtp-in-rcpt/edit")
            .insert(true)
            .create("DATA stage")
            .route("/smtp-in-data/edit")
            .insert(true)
            .create("Extensions")
            .route("/smtp-in-extensions/edit")
            .insert(true)
            .create("ASN & GeoIP")
            .route("/smtp-in-asn/edit")
            .insert(true)
            .create("MTA-STS")
            .route("/smtp-in-mta-sts/edit")
            .insert(true)
            .create("Session Limits")
            .route("/smtp-in-limits/edit")
            .insert(true)
            .create("Rate Limits")
            .route("/smtp-in-throttle")
            .insert(true)
            .create("Milters")
            .route("/milter")
            .insert(true)
            .create("MTA Hooks")
            .route("/mta-hooks")
            .insert(true)
            .insert(true)
            .create("Outbound")
            .create("Queue")
            .route("/smtp-out-queue/edit")
            .insert(true)
            .create("Routing")
            .route("/smtp-out-routing/edit")
            .insert(true)
            .create("TLS")
            .route("/smtp-out-tls/edit")
            .insert(true)
            .create("DNS")
            .route("/smtp-out-resolver/edit")
            .insert(true)
            .create("Limits")
            .route("/smtp-out-limits/edit")
            .insert(true)
            .create("Rate Limits")
            .route("/smtp-out-throttle")
            .insert(true)
            .create("Quotas")
            .route("/smtp-out-quota")
            .insert(true)
            .create("Relay Hosts")
            .route("/smtp-out-remote")
            .insert(true)
            .insert(true)
            .create("DKIM")
            .create("Settings")
            .route("/dkim/edit")
            .insert(true)
            .create("Signatures")
            .route("/signature")
            .insert(true)
            .insert(true)
            .create("ARC")
            .route("/arc/edit")
            .insert(true)
            .create("SPF")
            .route("/spf/edit")
            .insert(true)
            .create("DMARC")
            .route("/dmarc/edit")
            .insert(true)
            .create("Reporting")
            .route("/report/edit")
            .insert(true)
            .insert(true)
            // IMAP
            .create("IMAP & POP3")
            .icon(view! { <IconInbox/> })
            .create("Authentication")
            .route("/imap-auth/edit")
            .insert(true)
            .create("Folders")
            .route("/imap-folders/edit")
            .insert(true)
            .create("Protocol Limits")
            .route("/imap-limits/edit")
            .insert(true)
            .create("Rate Limits")
            .route("/imap-rate-limit/edit")
            .insert(true)
            .insert(true)
            // Security
            .create("Security")
            .icon(view! { <IconHandRaised/> })
            // Threat Shield
            .create("Automatic Ban")
            .route("/auto-ban/edit")
            .insert(true)
            // Blocked IPs
            .create("Blocked IPs")
            .route("/blocked-ip")
            .insert(true)
            // Blocked IPs
            .create("Allowed IPs")
            .route("/allowed-ip")
            .insert(true)
            .insert(true)
            // Telemetry
            .create("Telemetry")
            .icon(view! { <IconSignal/> })
            .create("Logging & Tracing")
            .route("/tracing")
            .insert(true)
            .create("Metrics")
            .route("/metrics/edit")
            .insert(true)
            .create("Alerts")
            .route("/alerts")
            .insert(true)
            .create("Webhooks")
            .route("/web-hooks")
            .insert(true)
            .create("Custom levels")
            .route("/custom-levels")
            .insert(true)
            .create("History")
            .route("/telemetry-history/edit")
            .insert(true)
            .insert(true)
            // SPAM Filter
            .create("Spam filter")
            .icon(view! { <IconShieldCheck/> })
            .create("Settings")
            .route("/spam-settings/edit")
            .insert(true)
            .create("Rules")
            .route("/spam-rule")
            .insert(true)
            .create("DNS blocklists")
            .route("/spam-dnsbl")
            .insert(true)
            .create("Bayes classifier")
            .route("/spam-bayes/edit")
            .insert(true)
            .create("LLM classifier")
            .route("/spam-llm/edit")
            .insert(true)
            .create("Pyzor")
            .route("/spam-pyzor/edit")
            .insert(true)
            .create("Reputation")
            .route("/spam-reputation/edit")
            .insert(true)
            .create("Scores")
            .route("/spam-score")
            .insert(true)
            .create("Lists")
            .create("Trusted domains")
            .route("/spam-trusted")
            .insert(true)
            .create("Blocked domains")
            .route("/spam-block")
            .insert(true)
            .create("Spam traps")
            .route("/spam-trap")
            .insert(true)
            .create("URL Redirectors")
            .route("/spam-redirect")
            .insert(true)
            .create("MIME Types")
            .route("/spam-mime")
            .insert(true)
            .insert(true)
            .insert(true)
            // Sieve Scripting
            .create("Scripting")
            .icon(view! { <IconCodeBracket/> })
            .create("Settings")
            .route("/sieve-settings/edit")
            .insert(true)
            .create("Limits")
            .route("/sieve-limits/edit")
            .insert(true)
            .create("System Scripts")
            .route("/trusted-script")
            .insert(true)
            .create("User Scripts")
            .route("/untrusted-script")
            .insert(true)
            .insert(true)
            .create("Management")
            .icon(view! { <IconServer/> })
            .raw_route(manage_url)
            .insert(true)
            .menu_items
    }
}

#[component]
pub fn Config() -> impl IntoView {
    let permissions = use_context::<Memo<Option<Permissions>>>().unwrap();
    let location = use_location();
    let path = create_memo(move |_| location.pathname.get());

    view! {
        <div class="max-w-[85rem] px-4 py-10 sm:px-6 lg:px-8 lg:py-14 mx-auto">
            <div class="grid sm:grid-cols-2 lg:grid-cols-3 gap-6">
                <div class="group flex flex-col h-full bg-white border border-gray-200 shadow-sm rounded-xl dark:bg-slate-900 dark:border-gray-700 dark:focus:outline-none dark:focus:ring-1 dark:focus:ring-gray-600">
                    <div class="h-52 flex flex-col justify-center items-center bg-gradient-to-br from-blue-600 to-blue-700 rounded-t-xl">
                        <IconAdjustmentsHorizontal class="size-28 text-white"/>
                    </div>
                    <div class="p-4 md:p-6">
                        <h3 class="text-xl font-semibold text-gray-800 dark:text-gray-300">
                            Layout Configuration
                        </h3>
                        <p class="mt-3 text-gray-500 dark:text-gray-400">
                            Customize the appearance of your web management interface by changing the logo and title
                        </p>
                    </div>
                    <div class="mt-auto flex border-t border-gray-200 divide-x divide-gray-200 dark:border-gray-700 dark:divide-gray-700">
                        <a
                            class="w-full py-3 px-4 inline-flex justify-center items-center gap-x-2 text-sm font-medium rounded-es-xl bg-white text-gray-800 shadow-sm hover:bg-gray-50 disabled:opacity-50 disabled:pointer-events-none dark:bg-slate-900 dark:border-gray-700 dark:text-white dark:hover:bg-gray-800 dark:focus:outline-none dark:focus:ring-1 dark:focus:ring-gray-600"
                            href="/settings/layout"
                        >
                            Configure
                        </a>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn LayoutConfigPage() -> impl IntoView {
    view! {
        <div class="max-w-[85rem] px-4 py-10 sm:px-6 lg:px-8 lg:py-14 mx-auto">
            <LayoutConfig/>
        </div>
    }
}
