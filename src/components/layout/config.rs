use gloo_storage::{LocalStorage, Storage};
use leptos::*;
use serde::{Deserialize, Serialize};
use web_sys::{File, FileReader, HtmlInputElement};
use crate::components::icon::{IconAdjustmentsHorizontal, IconPencilSquare, IconXMark, IconEye, IconEyeSlash, IconArrowUpTray, IconArrowPath, IconSpinner};
use crate::utils::storage::LocalStorage;
use crate::utils::validation::{validate_url, sanitize_input};
use crate::utils::security::{generate_csrf_token, validate_csrf_token, check_rate_limit};
use crate::utils::audit::{log_audit, AuditAction};

const LAYOUT_CONFIG_KEY: &str = "layout_config";
const MAX_TITLE_LENGTH: usize = 100;
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5MB
const ALLOWED_IMAGE_TYPES: [&str; 4] = ["image/jpeg", "image/png", "image/svg+xml", "image/gif"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub logo_url: String,
    pub title: String,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            logo_url: "/logo.svg".to_string(),
            title: "Stalwart Management".to_string(),
        }
    }
}

impl LayoutConfig {
    fn load() -> Self {
        LocalStorage::get::<LayoutConfig>(LAYOUT_CONFIG_KEY).unwrap_or_default()
    }

    fn save(&self) {
        LocalStorage::set(LAYOUT_CONFIG_KEY, self).unwrap();
    }

    fn validate(&self) -> Result<(), String> {
        if self.title.is_empty() {
            return Err("Title cannot be empty".to_string());
        }
        if self.title.len() > MAX_TITLE_LENGTH {
            return Err(format!("Title must be less than {} characters", MAX_TITLE_LENGTH));
        }
        if !self.logo_url.is_empty() && !validate_url(&self.logo_url) {
            return Err("Invalid logo URL".to_string());
        }
        Ok(())
    }
}

#[component]
pub fn LayoutConfig() -> impl IntoView {
    let (config, set_config) = create_signal(LayoutConfig::load());
    let (is_editing, set_is_editing) = create_signal(false);
    let (new_logo_url, set_new_logo_url) = create_signal(config.get().logo_url);
    let (new_title, set_new_title) = create_signal(config.get().title);
    let (error, set_error) = create_signal(String::new());
    let (show_preview, set_show_preview) = create_signal(false);
    let (is_uploading, set_is_uploading) = create_signal(false);
    let (auto_save, set_auto_save) = create_signal(true);
    let (is_saving, set_is_saving) = create_signal(false);
    let (csrf_token, set_csrf_token) = create_signal(generate_csrf_token());

    // 自动保存功能
    create_effect(move |_| {
        if auto_save.get() && is_editing.get() {
            if let Err(e) = check_rate_limit("layout_config_save") {
                set_error.set(e);
                return;
            }

            let logo_url = new_logo_url.get();
            let title = sanitize_input(&new_title.get());
            
            if !title.is_empty() {
                set_is_saving.set(true);
                let new_config = LayoutConfig {
                    logo_url: if logo_url.is_empty() { "/logo.svg".to_string() } else { logo_url },
                    title,
                };
                
                match new_config.validate() {
                    Ok(_) => {
                        set_config.set(new_config.clone());
                        new_config.save();
                        set_error.set(String::new());
                        log_audit(
                            AuditAction::ConfigUpdate,
                            "user", // 这里应该使用实际的用户名
                            &format!("Updated layout config: title={}", title),
                            None, // 这里应该使用实际的IP地址
                            true,
                        );
                    }
                    Err(e) => {
                        set_error.set(e);
                        log_audit(
                            AuditAction::ConfigUpdate,
                            "user",
                            &format!("Failed to update layout config: {}", e),
                            None,
                            false,
                        );
                    }
                }
                set_is_saving.set(false);
            }
        }
    });

    // Load config from storage on mount
    create_effect(move |_| {
        if let Ok(stored_config) = LocalStorage::get::<LayoutConfig>(LAYOUT_CONFIG_KEY) {
            set_config.set(stored_config);
        }
    });

    // Save config to storage when it changes
    create_effect(move |_| {
        if let Err(e) = LocalStorage::set(LAYOUT_CONFIG_KEY, config.get()) {
            log::error!("Failed to save layout config: {}", e);
        }
    });

    let handle_save = move |_| {
        if new_logo_url.get().is_empty() {
            set_error.set("Logo URL cannot be empty".to_string());
            return;
        }
        if new_title.get().is_empty() {
            set_error.set("Title cannot be empty".to_string());
            return;
        }
        set_error.set(String::new());
        set_config.set(LayoutConfig {
            logo_url: new_logo_url.get(),
            title: new_title.get(),
        });
        set_is_editing.set(false);
        set_show_preview.set(false);
    };

    let handle_preview = move |_| {
        if new_logo_url.get().is_empty() {
            set_error.set("Logo URL cannot be empty".to_string());
            return;
        }
        if new_title.get().is_empty() {
            set_error.set("Title cannot be empty".to_string());
            return;
        }
        set_error.set(String::new());
        set_show_preview.update(|v| *v = !*v);
    };

    let handle_file_upload = move |ev: web_sys::Event| {
        if let Err(e) = check_rate_limit("layout_config_upload") {
            set_error.set(e);
            return;
        }

        let input: HtmlInputElement = event_target(&ev).unwrap().dyn_into().unwrap();
        if let Some(file) = input.files().unwrap().get(0) {
            let file_type = file.type_();
            if !ALLOWED_IMAGE_TYPES.contains(&file_type.as_str()) {
                set_error.set("Invalid file type. Please upload a JPEG, PNG, SVG, or GIF image.".to_string());
                log_audit(
                    AuditAction::FileUpload,
                    "user",
                    "Attempted to upload invalid file type",
                    None,
                    false,
                );
                return;
            }

            if file.size() as usize > MAX_FILE_SIZE {
                set_error.set(format!("File size must be less than {}MB", MAX_FILE_SIZE / 1024 / 1024));
                log_audit(
                    AuditAction::FileUpload,
                    "user",
                    "Attempted to upload file exceeding size limit",
                    None,
                    false,
                );
                return;
            }

            set_is_uploading.set(true);
            let reader = FileReader::new().unwrap();
            let cloned_set_logo_url = set_new_logo_url.clone();
            let cloned_set_error = set_error.clone();
            let cloned_set_is_uploading = set_is_uploading.clone();

            reader.set_onload(Some(Box::new(move |_| {
                let result = reader.result().unwrap();
                if let Ok(data_url) = result.dyn_into::<js_sys::JsString>() {
                    let data_url = data_url.as_string().unwrap();
                    cloned_set_logo_url.set(data_url);
                    cloned_set_error.set(String::new());
                    log_audit(
                        AuditAction::FileUpload,
                        "user",
                        "Successfully uploaded new logo",
                        None,
                        true,
                    );
                } else {
                    cloned_set_error.set("Failed to read file".to_string());
                    log_audit(
                        AuditAction::FileUpload,
                        "user",
                        "Failed to read uploaded file",
                        None,
                        false,
                    );
                }
                cloned_set_is_uploading.set(false);
            }) as Box<dyn FnMut(_)>));

            reader.read_as_data_url(&file).unwrap();
        }
    };

    let handle_reset = move |_| {
        if let Err(e) = check_rate_limit("layout_config_reset") {
            set_error.set(e);
            return;
        }

        set_config.set(LayoutConfig::default());
        set_new_logo_url.set(LayoutConfig::default().logo_url);
        set_new_title.set(LayoutConfig::default().title);
        set_error.set(String::new());
        set_show_preview.set(false);
        set_csrf_token.set(generate_csrf_token());
        
        log_audit(
            AuditAction::ResetConfig,
            "user",
            "Reset layout configuration to default",
            None,
            true,
        );
    };

    view! {
        <div class="max-w-3xl mx-auto">
            <div class="bg-white shadow-sm rounded-xl dark:bg-slate-900 dark:border-gray-700">
                <div class="p-4 sm:p-7">
                    <div class="flex justify-between items-center mb-6">
                        <div class="flex items-center gap-x-3">
                            <IconAdjustmentsHorizontal class="size-6 text-gray-800 dark:text-gray-200"/>
                            <h2 class="text-xl font-semibold text-gray-800 dark:text-gray-200">
                                Layout Configuration
                            </h2>
                            <Show when=move || is_saving.get()>
                                <div class="flex items-center gap-x-2 text-sm text-gray-500 dark:text-gray-400">
                                    <IconSpinner class="size-4 animate-spin"/>
                                    "Saving..."
                                </div>
                            </Show>
                        </div>
                        <div class="flex items-center gap-x-2">
                            <button
                                class="inline-flex items-center gap-x-2 text-sm font-semibold rounded-lg border border-transparent text-gray-600 hover:text-gray-800 disabled:opacity-50 disabled:pointer-events-none dark:text-gray-400 dark:hover:text-gray-300 dark:focus:outline-none dark:focus:ring-1 dark:focus:ring-gray-600"
                                on:click=handle_reset
                            >
                                <IconArrowPath class="size-4"/>
                                "Reset to Default"
                            </button>
                            <button
                                class="inline-flex items-center gap-x-2 text-sm font-semibold rounded-lg border border-transparent text-blue-600 hover:text-blue-800 disabled:opacity-50 disabled:pointer-events-none dark:text-blue-500 dark:hover:text-blue-400 dark:focus:outline-none dark:focus:ring-1 dark:focus:ring-gray-600"
                                on:click=move |_| {
                                    set_is_editing.update(|v| *v = !*v);
                                    if !is_editing.get() {
                                        set_new_logo_url.set(config.get().logo_url);
                                        set_new_title.set(config.get().title);
                                        set_error.set(String::new());
                                        set_show_preview.set(false);
                                    }
                                }
                            >
                                {move || if is_editing.get() {
                                    view! {
                                        <IconXMark class="size-4"/>
                                        "Cancel"
                                    }
                                } else {
                                    view! {
                                        <IconPencilSquare class="size-4"/>
                                        "Edit"
                                    }
                                }}
                            </button>
                        </div>
                    </div>

                    <Show when=move || is_editing.get()>
                        <div class="space-y-4">
                            <div>
                                <label class="block text-sm font-medium mb-2 text-gray-800 dark:text-gray-200">
                                    Logo
                                </label>
                                <div class="flex gap-x-2">
                                    <input
                                        type="text"
                                        class="py-3 px-4 block w-full border-gray-200 rounded-lg text-sm focus:border-blue-500 focus:ring-blue-500 disabled:opacity-50 disabled:pointer-events-none dark:bg-slate-900 dark:border-gray-700 dark:text-gray-400 dark:focus:ring-gray-600"
                                        value=new_logo_url.get()
                                        on:input=move |ev| set_new_logo_url.set(event_target_value(&ev))
                                    />
                                    <label class="py-3 px-4 inline-flex items-center gap-x-2 text-sm font-semibold rounded-lg border border-gray-200 text-gray-800 hover:bg-gray-100 disabled:opacity-50 disabled:pointer-events-none dark:border-gray-700 dark:text-white dark:hover:bg-gray-700 dark:focus:outline-none dark:focus:ring-1 dark:focus:ring-gray-600 cursor-pointer">
                                        <IconArrowUpTray class="size-4"/>
                                        "Upload"
                                        <input
                                            type="file"
                                            class="hidden"
                                            accept="image/*"
                                            on:change=handle_file_upload
                                        />
                                    </label>
                                </div>
                                <p class="mt-2 text-sm text-gray-500 dark:text-gray-400">
                                    Enter the URL of your logo image or upload a new one. Supported formats: PNG, JPG, SVG.
                                </p>
                            </div>
                            <div>
                                <label class="block text-sm font-medium mb-2 text-gray-800 dark:text-gray-200">
                                    Title
                                </label>
                                <input
                                    type="text"
                                    class="py-3 px-4 block w-full border-gray-200 rounded-lg text-sm focus:border-blue-500 focus:ring-blue-500 disabled:opacity-50 disabled:pointer-events-none dark:bg-slate-900 dark:border-gray-700 dark:text-gray-400 dark:focus:ring-gray-600"
                                    value=new_title.get()
                                    on:input=move |ev| set_new_title.set(event_target_value(&ev))
                                />
                                <p class="mt-2 text-sm text-gray-500 dark:text-gray-400">
                                    This will be displayed in the browser tab and as the main title of the application.
                                </p>
                            </div>

                            <Show when=move || !error.get().is_empty()>
                                <div class="p-4 text-sm text-red-800 rounded-lg bg-red-50 dark:bg-gray-800 dark:text-red-400" role="alert">
                                    {move || error.get()}
                                </div>
                            </Show>

                            <div class="mt-5 flex justify-end gap-x-2">
                                <button
                                    class="py-3 px-4 inline-flex items-center gap-x-2 text-sm font-semibold rounded-lg border border-gray-200 text-gray-800 hover:bg-gray-100 disabled:opacity-50 disabled:pointer-events-none dark:border-gray-700 dark:text-white dark:hover:bg-gray-700 dark:focus:outline-none dark:focus:ring-1 dark:focus:ring-gray-600"
                                    on:click=handle_preview
                                >
                                    {move || if show_preview.get() {
                                        view! {
                                            <IconEyeSlash class="size-4"/>
                                            "Hide Preview"
                                        }
                                    } else {
                                        view! {
                                            <IconEye class="size-4"/>
                                            "Show Preview"
                                        }
                                    }}
                                </button>
                                <button
                                    class="py-3 px-4 inline-flex items-center gap-x-2 text-sm font-semibold rounded-lg border border-transparent bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:pointer-events-none dark:focus:outline-none dark:focus:ring-1 dark:focus:ring-gray-600"
                                    on:click=handle_save
                                    disabled=move || is_uploading.get()
                                >
                                    {move || if is_uploading.get() {
                                        "Uploading..."
                                    } else {
                                        "Save Changes"
                                    }}
                                </button>
                            </div>

                            <Show when=move || show_preview.get()>
                                <div class="mt-6 p-4 border border-gray-200 rounded-lg dark:border-gray-700">
                                    <h3 class="text-sm font-medium text-gray-800 dark:text-gray-200 mb-4">
                                        Preview
                                    </h3>
                                    <div class="space-y-4">
                                        <div class="flex items-center gap-x-3">
                                            <img
                                                src=move || new_logo_url.get()
                                                class="h-8"
                                                alt="Logo preview"
                                            />
                                            <span class="text-lg font-semibold text-gray-800 dark:text-gray-200">
                                                {move || new_title.get()}
                                            </span>
                                        </div>
                                        <div class="text-sm text-gray-500 dark:text-gray-400">
                                            Browser tab title: {move || new_title.get()}
                                        </div>
                                    </div>
                                </div>
                            </Show>
                        </div>
                    </Show>

                    <Show when=move || !is_editing.get()>
                        <div class="space-y-6">
                            <div>
                                <h3 class="text-sm font-medium text-gray-800 dark:text-gray-200 mb-2">
                                    Current Logo
                                </h3>
                                <div class="p-4 bg-gray-50 rounded-lg dark:bg-gray-800">
                                    <img
                                        src=move || config.get().logo_url
                                        class="h-12 mx-auto"
                                        alt="Current logo"
                                    />
                                </div>
                                <p class="mt-2 text-sm text-gray-500 dark:text-gray-400">
                                    {move || config.get().logo_url}
                                </p>
                            </div>
                            <div>
                                <h3 class="text-sm font-medium text-gray-800 dark:text-gray-200 mb-2">
                                    Current Title
                                </h3>
                                <div class="p-4 bg-gray-50 rounded-lg dark:bg-gray-800">
                                    <p class="text-lg font-semibold text-gray-800 dark:text-gray-200">
                                        {move || config.get().title}
                                    </p>
                                </div>
                            </div>
                        </div>
                    </Show>

                    <div class="mt-6">
                        <div class="flex items-center gap-x-2 mb-4">
                            <input
                                type="checkbox"
                                id="auto-save"
                                class="size-4 border-gray-300 rounded text-blue-600 focus:ring-blue-500 dark:bg-slate-900 dark:border-gray-700 dark:checked:bg-blue-500 dark:checked:border-blue-500 dark:focus:ring-offset-gray-800"
                                checked=auto_save
                                on:change=move |ev| set_auto_save.set(event_target_checked(&ev))
                            />
                            <label for="auto-save" class="text-sm text-gray-600 dark:text-gray-400">
                                "Auto-save changes"
                            </label>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
} 