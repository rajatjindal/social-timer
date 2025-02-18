use components::{Route, Router, Routes};
use leptos::{prelude::*, task::spawn_local};
use leptos_meta::*;
use leptos_router::*;

#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() root="" />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let fallback = || view! { "Page not found." }.into_view();

    view! {
        <Stylesheet id="leptos" href="/pkg/social_timer.css" />
        <Meta
            name="description"
            content="A website running its server-side as a WASI Component :D"
        />

        <Title text="Social Timer" />

        <Router>
            <main>
                <Routes fallback>
                    <Route path=path!("") view=HomePage />
                    <Route path=path!("/*any") view=NotFound />
                </Routes>
            </main>
        </Router>
    }
}

pub struct ElapsedTime {
    years: u64,
    months: u64,
    days: u64,
    hours: u64,
    minutes: u64,
    seconds: u64,
}

impl ElapsedTime {
    fn get_elapsed_time(seconds: u64) -> Self {
        let years = seconds / 31536000;
        let months = (seconds % 31536000) / 2592000;
        let days = ((seconds % 31536000) % 2592000) / 86400;
        let hours = (((seconds % 31536000) % 2592000) % 86400) / 3600;
        let minutes = ((((seconds % 31536000) % 2592000) % 86400) % 3600) / 60;
        let seconds = ((((seconds % 31536000) % 2592000) % 86400) % 3600) % 60;

        ElapsedTime {
            years,
            months,
            days,
            hours,
            minutes,
            seconds,
        }
    }
}

fn current_epoch() -> u64 {
    wasm_timer::SystemTime::now()
        .duration_since(wasm_timer::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let (last_update, set_last_update) = signal(current_epoch());
    let current_count = Resource::new(
        move || last_update.get(),
        |last| async move {
            println!("Getting value via resource");
            get_count(last).await
        },
    );

    // Creates a reactive value to update the button
    let (count, set_count) = signal::<u64>(0);

    use_interval(1000, move || {
        let epoch = current_epoch();
        set_count.set(epoch);
    });

    let on_click = move |_| {
        println!("Button clicked");
        spawn_local(async move {
            let current_epoch = current_epoch();
            reset_count(current_epoch).await.unwrap();
            set_last_update.set(current_epoch);
            set_count.set(0);
        });
    };

    view! {
        <h1 class="title">"Sekunden ohne LinkedIn Vorschlag"</h1>
        {move || {
            match current_count.get() {
                Some(last_update_result) => {
                    let lu2 = last_update_result.unwrap();
                
                    view! {
                        // something is off here, the seconds are not updating without
                        // the p tag.
                        <ElapsedTimeDisp seconds={move ||  count.get() - lu2 } />
                        <p style="display: none"> {format!("{} Sekunden", count.get() - lu2)} </p>
                        <button on:click=on_click>"Ich habe einen Vorschlag!"</button>
                    }
                        .into_any()
                }
                None => view! { <p>"Loading value"</p> }.into_any(),
            }
        }}
    }
}

#[component]
fn ElapsedTimeDisp(seconds: impl Fn() -> u64 + Send + Sync + 'static) -> impl IntoView {
    view! {
        <h1 class="seconds" inner_html={
            let et = ElapsedTime::get_elapsed_time(seconds());
            format!(
                "{}&nbsp;Jahre, {}&nbsp;Monate, {}&nbsp;Tage, {}&nbsp;Stunden, {}&nbsp;Minuten und {}&nbsp;Sekunden.",
                et.years, et.months, et.days, et.hours, et.minutes, et.seconds)} >
        </h1>
    }
}

/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    // set an HTTP status code 404
    // this is feature gated because it can only be done during
    // initial server-side rendering
    // if you navigate to the 404 page subsequently, the status
    // code will not be set because there is not a new HTTP request
    // to the server
    #[cfg(feature = "ssr")]
    {
        // this can be done inline because it's synchronous
        // if it were async, we'd use a server function
        if let Some(resp) = use_context::<leptos_wasi::response::ResponseOptions>() {
            resp.set_status(leptos_wasi::prelude::StatusCode::NOT_FOUND);
        }
    }

    view! { <h1>"Not Found"</h1> }
}

#[server(prefix = "/api")]
pub async fn get_count(ep: u64) -> Result<u64, ServerFnError<String>> {
    println!("Getting value");
    let store = spin_sdk::key_value::Store::open_default().map_err(|e| e.to_string())?;
    let count = store.get_json::<u64>("social_timer_count");

    match count {
        Ok(Some(c)) => {
            println!("Got value {}", c);
            Ok(c)
        }
        Ok(None) => {
            println!("No value found, resetting to 0");
            reset_count(ep).await.expect("Cannot reset counter");
            Ok(ep)
        }
        Err(e) => {
            println!("Error getting value {} , resetting value to {}", e, ep);
            reset_count(ep).await.expect("Cannot reset counter");
            Ok(ep)
        }
    }
}

#[server(prefix = "/api")]
pub async fn reset_count(counter: u64) -> Result<u64, ServerFnError<String>> {
    println!("Resetting value");
    let store = spin_sdk::key_value::Store::open_default().map_err(|e| e.to_string())?;
    store
        .set_json("social_timer_count", &counter)
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    Ok(counter)
}

/// Hook to wrap the underlying `setInterval` call and make it reactive w.r.t.
/// possible changes of the timer interval.
pub fn use_interval<T, F>(interval_millis: T, f: F)
where
    F: Fn() + Clone + 'static,
    T: Into<Signal<u64>> + 'static,
{
    let interval_millis = interval_millis.into();
    Effect::new(move |prev_handle: Option<IntervalHandle>| {
        // effects get their previous return value as an argument
        // each time the effect runs, it will return the interval handle
        // so if we have a previous one, we cancel it
        if let Some(prev_handle) = prev_handle {
            prev_handle.clear();
        };

        // here, we return the handle
        set_interval_with_handle(
            f.clone(),
            // this is the only reactive access, so this effect will only
            // re-run when the interval changes
            std::time::Duration::from_millis(interval_millis.get()),
        )
        .expect("could not create interval")
    });
}
