use chrono::{Datelike, Local, NaiveDate};
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::query_signal;
use leptos_router::path;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use thousands::Separable;
use urlencoding::decode;
use urlencoding::encode;

mod date_util;
mod fin;
mod input;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
struct FormData {
    loan_amt: Decimal,
    int_yearly_pct: Decimal,
    num_pmts: u64,
    start_date: NaiveDate,
}

/// The form fields exactly as typed, before validation.
#[derive(Clone, Debug, PartialEq)]
struct RawInput {
    loan_amt: String,
    int_yearly_pct: String,
    num_pmts: String,
    start_date: NaiveDate,
}

impl RawInput {
    fn parse(&self) -> Result<FormData, String> {
        let li = input::parse_loan_input(&self.loan_amt, &self.int_yearly_pct, &self.num_pmts)?;
        Ok(FormData {
            loan_amt: li.loan_amt,
            int_yearly_pct: li.int_yearly_pct,
            num_pmts: li.num_pmts,
            start_date: self.start_date,
        })
    }
}

impl From<FormData> for RawInput {
    fn from(fd: FormData) -> Self {
        Self {
            loan_amt: format!("{:.0}", fd.loan_amt),
            int_yearly_pct: fd.int_yearly_pct.normalize().to_string(),
            num_pmts: fd.num_pmts.to_string(),
            start_date: fd.start_date,
        }
    }
}

/// Decodes the shared-link state; anything malformed or invalid is ignored.
fn decode_url_data(url_data: &str) -> Option<FormData> {
    let decoded = decode(url_data).ok()?;
    let fd: FormData = serde_json::from_str(&decoded).ok()?;
    RawInput::from(fd).parse().ok()
}

fn encode_url_data(fd: &FormData) -> Option<String> {
    serde_json::to_string(fd)
        .ok()
        .map(|json| encode(&json).into_owned())
}

const REPO_URL: &str = "https://github.com/societymartingale/amort-calc";

/// Footer text and optional commit link, from build-time env vars set in CI.
fn build_info(tag: Option<&str>, sha: Option<&str>) -> (String, Option<String>) {
    let tag = tag.map(str::trim).filter(|t| !t.is_empty());
    let sha = sha.map(str::trim).filter(|s| !s.is_empty());
    let id = match (tag, sha) {
        (Some(t), _) => t.to_string(),
        (None, Some(s)) => s.chars().take(7).collect(),
        (None, None) => "dev".to_string(),
    };
    let url = sha.map(|s| format!("{REPO_URL}/commit/{s}"));
    (format!("v{} ({id})", env!("CARGO_PKG_VERSION")), url)
}

#[component]
fn Footer() -> impl IntoView {
    let (label, url) = build_info(option_env!("APP_BUILD_TAG"), option_env!("APP_BUILD_SHA"));
    view! {
        <footer class="version">
            {match url {
                Some(href) => view! { <a href=href>{label}</a> }.into_any(),
                None => label.into_any(),
            }}
        </footer>
    }
}

#[component]
pub fn Home() -> impl IntoView {
    view! {
      <Router>
        <Routes fallback=move || view! { <PageNotFound /> }>
          <Route path=path!("") view=move || {
            view! { <App/> }
          }/>
        </Routes>
      </Router>
    }
}

#[component]
pub fn PageNotFound() -> impl IntoView {
    view! {
        <p>Page not found</p>
    }
}

#[component]
fn App() -> impl IntoView {
    let (url_data, set_url_data) = query_signal::<String>("input");
    let cur_date = Local::now().date_naive();
    let initial = url_data
        .get_untracked()
        .and_then(|u| decode_url_data(&u))
        .unwrap_or(FormData {
            loan_amt: Decimal::from(500_000),
            int_yearly_pct: Decimal::from(6),
            num_pmts: 360,
            start_date: cur_date.with_day(1).unwrap_or(cur_date),
        });

    let raw = RwSignal::new(RawInput::from(initial));
    let parsed = Memo::new(move |_| raw.with(RawInput::parse));
    // Keep the shareable URL in sync with the last valid input.
    Effect::new(move |_| {
        if let Ok(fd) = parsed.get() {
            set_url_data.set(encode_url_data(&fd));
        }
    });

    let results = move || match parsed.get() {
        Err(msg) => view! { <p class="error" role="alert">{msg}</p> }.into_any(),
        Ok(fd) => {
            let int_monthly = fd.int_yearly_pct / Decimal::from(1200);
            let pmt =
                fin::checked_monthly_pmt(fd.loan_amt, int_monthly, fd.num_pmts).unwrap_or_default();
            let rows = fin::calc_amort_sched(fd.loan_amt, int_monthly, pmt, fd.num_pmts);
            let dates = date_util::payment_dates(fd.start_date, fd.num_pmts);
            let total_int = fin::total_interest(&rows);
            view! {
                <p>
                    Total interest paid over life of loan:
                    <span class="highlight">
                        {format!("{:.2}", total_int).separate_with_commas()}
                    </span>
                </p>
                <AmortSchedTable rows=rows dates=dates/>
            }
            .into_any()
        }
    };

    view! {
        <div class="app">
            <Form raw=raw/>
            {results}
            <Footer/>
        </div>
    }
}

#[component]
fn Form(raw: RwSignal<RawInput>) -> impl IntoView {
    const INT_YEARLY_PCT: &str = "int_yearly_pct";
    const LOAN_AMT: &str = "loan_amt";
    const NUM_PMTS: &str = "num_pmts";
    const TEXT: &str = "text";

    let init = raw.get_untracked();

    view! {
        <form>
            <label for=LOAN_AMT>Loan Amount: </label>
            <input type=TEXT id=LOAN_AMT name=LOAN_AMT value=init.loan_amt size="12"
                on:input=move |ev| raw.update(|r| r.loan_amt = event_target_value(&ev))
            />
            <label for=INT_YEARLY_PCT>Annual Percentage Rate: </label>
            <input type=TEXT id=INT_YEARLY_PCT name=INT_YEARLY_PCT value=init.int_yearly_pct size="4"
                on:input=move |ev| raw.update(|r| r.int_yearly_pct = event_target_value(&ev))
            />
            <label for=NUM_PMTS>Number of Monthly Payments: </label>
            <input type=TEXT id=NUM_PMTS name=NUM_PMTS value=init.num_pmts size="4" maxlength="4"
                on:input=move |ev| raw.update(|r| r.num_pmts = event_target_value(&ev))
            />
            <label for="start">Loan start date: </label>
            <input type="date" id="start" name="loan-start" value=init.start_date.to_string()
                on:input=move |ev| {
                    if let Ok(d) = NaiveDate::parse_from_str(&event_target_value(&ev), "%Y-%m-%d") {
                        raw.update(|r| r.start_date = d);
                    }
                }
            />
        </form>
    }
}

#[component]
fn AmortSchedTable(rows: Vec<fin::AmortRow>, dates: Vec<NaiveDate>) -> impl IntoView {
    const COLS: [&str; 7] = [
        "Period",
        "Payment Date",
        "Beginning Balance",
        "Payment",
        "Interest",
        "Principal",
        "Ending Balance",
    ];
    view! {
        <table>
            <tr>
                {COLS.into_iter()
                    .map(|c| view! { <th>{c}</th>})
                    .collect::<Vec<_>>()}
           </tr>
       {rows.into_iter().map(
            |r| view! {
                <tr>
                    <td>{r.period}</td>
                    <td>{usize::try_from(r.period - 1).ok().and_then(|i| dates.get(i)).map(|d| d.to_string()).unwrap_or_default()}</td>
                    <td>{format!("{:.2}", r.beg_bal.round_dp(2)).separate_with_commas()}</td>
                    <td>{format!("{:.2}", r.pmt.round_dp(2)).separate_with_commas()}</td>
                    <td>{format!("{:.2}", r.int_pmt.round_dp(2)).separate_with_commas()}</td>
                    <td>{format!("{:.2}", r.princ_pmt.round_dp(2)).separate_with_commas()}</td>
                    <td>{format!("{:.2}", r.end_bal.round_dp(2)).separate_with_commas()}</td>
                </tr>}).collect::<Vec<_>>()}
        </table>
    }
}

fn main() {
    leptos::mount::mount_to_body(Home)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_info_variants() {
        let v = env!("CARGO_PKG_VERSION");
        assert_eq!(build_info(None, None), (format!("v{v} (dev)"), None));
        assert_eq!(
            build_info(Some(""), Some(" ")),
            (format!("v{v} (dev)"), None)
        );
        let sha = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(
            build_info(None, Some(sha)),
            (
                format!("v{v} (0123456)"),
                Some(format!("{REPO_URL}/commit/{sha}"))
            )
        );
        assert_eq!(
            build_info(Some("v1.2.0"), Some(sha)).0,
            format!("v{v} (v1.2.0)")
        );
    }

    #[test]
    fn url_data_garbage_is_ignored() {
        assert!(decode_url_data("").is_none());
        assert!(decode_url_data("not-json").is_none());
        assert!(decode_url_data("%E0%A4%A").is_none());
        // well-formed JSON but invalid terms (0 payments)
        let bad = encode(
            r#"{"loan_amt":"1000","int_yearly_pct":"5","num_pmts":0,"start_date":"2024-01-01"}"#,
        );
        assert!(decode_url_data(&bad).is_none());
    }

    #[test]
    fn url_data_round_trips() {
        let fd = FormData {
            loan_amt: Decimal::from(250_000),
            int_yearly_pct: Decimal::from_str("6.25").unwrap(),
            num_pmts: 180,
            start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
        };
        let back = decode_url_data(&encode_url_data(&fd).unwrap()).unwrap();
        assert_eq!(back.loan_amt, fd.loan_amt);
        assert_eq!(back.int_yearly_pct, fd.int_yearly_pct);
        assert_eq!(back.num_pmts, fd.num_pmts);
        assert_eq!(back.start_date, fd.start_date);
    }
}
