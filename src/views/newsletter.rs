use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::tcp_client;

#[server]
async fn add_newsletter_subscriber(email: String) -> Result<String, ServerFnError> {
    let email = crate::newsletter::normalize_email(&email)
        .ok_or_else(|| ServerFnError::new("Please provide a valid email address."))?;

    let cmd = format!("[CMD] add newsletter={}", email);
    tcp_client::send_command(&cmd)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to add newsletter subscriber: {}", e)))?;

    let encoding_key = std::env::var("ENCODING")
        .map_err(|_| ServerFnError::new("ENCODING is not configured on this server."))?;
    let link = crate::newsletter::build_unsubscribe_link(
        &email,
        &crate::newsletter::newsletter_base_url(),
        &encoding_key,
    )
    .map_err(ServerFnError::new)?;

    Ok(link)
}

pub fn Newsletter() -> Element {
    let mut email = use_signal(String::new);
    let mut status = use_signal(|| Option::<String>::None);
    let mut submitting = use_signal(|| false);
    let mut submit_newsletter = move || {
        if submitting() {
            return;
        }

        let value = email().trim().to_string();
        if value.is_empty() {
            status.set(Some("Enter an email address first.".to_string()));
            return;
        }

        submitting.set(true);
        status.set(None);
        spawn(async move {
            let message = match add_newsletter_subscriber(value).await {
                Ok(link) => format!("Subscribed. Personal unsubscribe link: {}", link),
                Err(err) => format!("Subscribe failed: {}", err),
            };
            status.set(Some(message));
            submitting.set(false);
        });
    };

    rsx! {
        section {
            class: "min-h-screen w-full bg-slate-900 text-white px-4 py-10",
            div {
                class: "mx-auto w-full max-w-2xl rounded-xl border border-slate-700 bg-slate-800 p-6 md:p-8 shadow-lg space-y-5",
                h1 { class: "text-2xl md:text-3xl font-semibold", "Newsletter" }
                p {
                    class: "text-slate-300",
                    "Subscribe for birdhouse updates. You will receive your personal encrypted unsubscribe link."
                }
                form {
                    id: "newsletter-form",
                    class: "flex flex-col sm:flex-row gap-3",
                    autocomplete: "on",
                    onsubmit: move |evt| {
                        evt.prevent_default();
                        submit_newsletter();
                    },
                    input {
                        id: "newsletter-email-input",
                        r#type: "email",
                        value: email(),
                        placeholder: "email@domain.com",
                        name: "email",
                        inputmode: "email",
                        autocomplete: "email",
                        autocapitalize: "none",
                        autocorrect: "off",
                        spellcheck: "false",
                        class: "w-full rounded-md bg-white text-black px-3 py-2 pr-12",
                        oninput: move |evt| email.set(evt.value()),
                        onkeydown: move |evt: KeyboardEvent| {
                            if evt.key().to_string().as_str() == "Enter" {
                                evt.prevent_default();
                                submit_newsletter();
                            }
                        },
                    }
                    button {
                        r#type: "submit",
                        class: format!(
                            "rounded-md px-4 py-2 font-medium {}",
                            if submitting() {
                                "bg-slate-500 text-white"
                            } else {
                                "bg-emerald-500 hover:bg-emerald-600 text-white"
                            }
                        ),
                        disabled: submitting(),
                        if submitting() { "Adding..." } else { "Subscribe" }
                    }
                }
                if let Some(message) = status() {
                    p { class: "text-sm text-slate-200 break-all", "{message}" }
                }
            }
        }
    }
}
