use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::tcp_client;

#[server]
async fn remove_newsletter_subscriber_by_token(encoded_email: String) -> Result<(), ServerFnError> {
    let encoding_key = std::env::var("ENCODING")
        .map_err(|_| ServerFnError::new("ENCODING is not configured on this server."))?;

    let decoded = crate::newsletter::decode_email(&encoded_email, &encoding_key)
        .map_err(ServerFnError::new)?;
    let email = crate::newsletter::normalize_email(&decoded)
        .ok_or_else(|| ServerFnError::new("Invalid unsubscribe link."))?;

    let cmd = format!("[CMD] remove newsletter={}", email);
    tcp_client::send_command(&cmd).await.map_err(|e| {
        ServerFnError::new(format!("Failed to remove newsletter subscriber: {}", e))
    })?;

    Ok(())
}

#[component]
pub fn Unsubscribe(encoded_email: String) -> Element {
    let token = encoded_email.clone();
    let status = use_resource(move || {
        let token = token.clone();
        async move { remove_newsletter_subscriber_by_token(token).await }
    });

    let message = match &*status.read_unchecked() {
        Some(Ok(_)) => "You are unsubscribed.".to_string(),
        Some(Err(err)) => format!("Unsubscribe failed: {}", err),
        None => "Processing unsubscribe link...".to_string(),
    };
    rsx! {
        section {
            class: "min-h-screen w-full bg-slate-900 text-white px-4 py-10",
            div {
                class: "mx-auto w-full max-w-2xl rounded-xl border border-slate-700 bg-slate-800 p-6 md:p-8 shadow-lg space-y-5",
                h1 { class: "text-2xl md:text-3xl font-semibold", "Newsletter" }
                p {
                    class: "text-slate-200",
                    "{message}"
                }
            }
        }
    }
}
