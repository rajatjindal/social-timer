use std::fmt::Debug;

use components::{Route, Router, Routes};
use leptos::logging::log;
use leptos::{prelude::*, task::spawn_local};
use leptos_meta::*;
use leptos_router::*;
use leptos_use::*;

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct ElapsedTime {
    years: u64,
    months: u64,
    days: u64,
    hours: u64,
    minutes: u64,
    seconds: u64,
}

enum TimeUnit {
    Years,
    Months,
    Days,
    Hours,
    Minutes,
    Seconds,
}

impl TimeUnit {
    fn format_timeunit(&self, value: u64) -> String {
        match self {
            TimeUnit::Years if value == 1 => format!("{}&nbsp;Jahr", value),
            TimeUnit::Years => format!("{}&nbsp;Jahre", value),
            TimeUnit::Months if value == 1 => format!("{}&nbsp;Monat", value),
            TimeUnit::Months => format!("{}&nbsp;Monate", value),
            TimeUnit::Days if value == 1 => format!("{}&nbsp;Tag", value),
            TimeUnit::Days => format!("{}&nbsp;Tage", value),
            TimeUnit::Hours if value == 1 => format!("{}&nbsp;Stunde", value),
            TimeUnit::Hours => format!("{}&nbsp;Stunden", value),
            TimeUnit::Minutes if value == 1 => format!("{}&nbsp;Minute", value),
            TimeUnit::Minutes => format!("{}&nbsp;Minuten", value),
            TimeUnit::Seconds if value == 1 => format!("{}&nbsp;Sekunde", value),
            TimeUnit::Seconds => format!("{}&nbsp;Sekunden", value),
        }
    }
}

impl ElapsedTime {
    const SECONDS_IN_YEAR: u64 = 31536000;
    const SECONDS_IN_MONTH: u64 = 2592000;
    const SECONDS_IN_DAY: u64 = 86400;
    const SECONDS_IN_HOUR: u64 = 3600;

    fn get_elapsed_time(seconds: u64) -> Self {
        let years = seconds / Self::SECONDS_IN_YEAR;
        let months = (seconds % Self::SECONDS_IN_YEAR) / Self::SECONDS_IN_MONTH;
        let days =
            ((seconds % Self::SECONDS_IN_YEAR) % Self::SECONDS_IN_MONTH) / Self::SECONDS_IN_DAY;
        let hours = (((seconds % Self::SECONDS_IN_YEAR) % Self::SECONDS_IN_MONTH)
            % Self::SECONDS_IN_DAY)
            / Self::SECONDS_IN_HOUR;
        let minutes = ((((seconds % Self::SECONDS_IN_YEAR) % Self::SECONDS_IN_MONTH)
            % Self::SECONDS_IN_DAY)
            % Self::SECONDS_IN_HOUR)
            / 60;
        let seconds = ((((seconds % Self::SECONDS_IN_YEAR) % Self::SECONDS_IN_MONTH)
            % Self::SECONDS_IN_DAY)
            % Self::SECONDS_IN_HOUR)
            % 60;

        ElapsedTime {
            years,
            months,
            days,
            hours,
            minutes,
            seconds,
        }
    }

    fn fmt_output(&self) -> String {
        format!(
            "{}, {}, {}, {}, {} und {}.",
            TimeUnit::Years.format_timeunit(self.years),
            TimeUnit::Months.format_timeunit(self.months),
            TimeUnit::Days.format_timeunit(self.days),
            TimeUnit::Hours.format_timeunit(self.hours),
            TimeUnit::Minutes.format_timeunit(self.minutes),
            TimeUnit::Seconds.format_timeunit(self.seconds)
        )
    }
}

/// retrieve the current epoch time in seconds.
fn current_epoch() -> u64 {
    wasm_timer::SystemTime::now()
        .duration_since(wasm_timer::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" type="image/x-icon" href="/static/favicon.ico" />
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
                    <Route path=path!("/submit") view=Submit />
                    <Route path=path!("/*any") view=NotFound />
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Initialize the count
    let epoch = current_epoch();

    // when did the last update happen (submission to server)
    let (last_update, set_last_update) = signal(current_epoch());

    // increasing value of the counter
    let (count, set_count) = signal::<u64>(epoch);

    let last_update_resource = Resource::new(
        move || last_update.get(),
        |last| async move {
            log!("Getting value via resource");
            get_count(last).await
        },
    );

    // update every second
    use_interval(1000, move || {
        let epoch = current_epoch();
        set_count(epoch);
    });


    let UseEventSourceReturn {
        ready_state, data, error, close, ..
    } =  use_event_source::<u64, codee::string::FromToStringCodec>("http:://localhost:3000/sse");

    // click handler set last_update to now
    let on_click = move |_| {
        spawn_local(async move {
            let current_epoch = current_epoch();
            reset_count(current_epoch).await.unwrap();
            set_last_update(current_epoch);
            set_count(current_epoch);
        });
    };

    view! {
        <h1 class="title">
            "Sekunden ohne "<img class="logo" src="/static/LI-Logo.png" width="15%" /> "Vorschlag"
        </h1>
        {move || {
            match last_update_resource.get() {
                Some(resource_result) => {
                    let lu2 = resource_result.unwrap();

                    view! {
                        <ElapsedTimeDisp seconds=count last_update=lu2></ElapsedTimeDisp>
                        <button on:click=on_click>"Ich habe einen Vorschlag!"</button>
                    }
                        .into_any()
                }
                None => view! { <h1 class="seconds">"Loading value"</h1> }.into_any(),
            }
        }}
    }
}

#[component]
fn ElapsedTimeDisp(seconds: ReadSignal<u64>, last_update: u64) -> impl IntoView {
    let et = move || ElapsedTime::get_elapsed_time(seconds.get() - last_update);
    view! { <h1 class="seconds" inner_html=move || et().fmt_output()></h1> }
}

#[component]
fn Submit() -> impl IntoView {
    view! {
        <div class="dialog">
            <h1>"Submit"</h1>
            <input type="text" />
            <input type="text" />
        </div>
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

/// Get the last update value from the server.
#[server(prefix = "/api")]
pub async fn get_count(ep: u64) -> Result<u64, ServerFnError<String>> {
    log!("Getting value from server");
    let store = spin_sdk::key_value::Store::open_default().map_err(|e| e.to_string())?;
    let count = store.get_json::<u64>("social_timer_count");

    match count {
        Ok(Some(c)) => {
            log!("Found value on server {}", c);
            Ok(c)
        }
        Ok(None) => {
            log!("No value on server found, resetting to {}", ep);
            reset_count(ep).await.expect("Cannot reset counter");
            Ok(ep)
        }
        Err(e) => {
            log!("Error getting value {} , resetting value to {}", e, ep);
            reset_count(ep).await.expect("Cannot reset counter");
            Ok(ep)
        }
    }
}

/// Reset the counter to a new value on the server.
/// This function is called when the button is clicked.
/// The new value is the current epoch time.
#[server(prefix = "/api")]
pub async fn reset_count(counter: u64) -> Result<u64, ServerFnError<String>> {
    log!("Resetting value on server");
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
