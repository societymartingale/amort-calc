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

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
struct FormData {
    loan_amt: Decimal,
    int_yearly_pct: Decimal,
    num_pmts: u64,
    start_date: NaiveDate,
}

impl FormData {
    pub fn new(
        loan_amt: Decimal,
        int_yearly_pct: Decimal,
        num_pmts: u64,
        start_date: NaiveDate,
    ) -> Self {
        Self {
            loan_amt,
            int_yearly_pct,
            num_pmts,
            start_date,
        }
    }
}

fn decode_url_data(url_data: &str) -> Option<FormData> {
    if !url_data.is_empty() {
        let decoded = decode(url_data).unwrap().into_owned();
        let fd: FormData = serde_json::from_str(&decoded).unwrap();
        return Some(fd);
    }
    None
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
    let mut loan_amt = Decimal::from_str("500000").unwrap();
    let mut int_yearly_pct = Decimal::from_str("6").unwrap();
    let mut num_pmts = 360;
    let mut start_date = NaiveDate::from_ymd_opt(cur_date.year(), cur_date.month(), 1).unwrap();
    let input = url_data.get_untracked().unwrap_or_default();
    if let Some(fd) = decode_url_data(&input) {
        loan_amt = fd.loan_amt;
        int_yearly_pct = fd.int_yearly_pct;
        num_pmts = fd.num_pmts;
        start_date = fd.start_date;
    }

    let (form_data, set_form_data) = signal(FormData::new(
        loan_amt,
        int_yearly_pct,
        num_pmts,
        start_date,
    ));
    let int_monthly =
        Memo::new(move |_| (form_data().int_yearly_pct) / Decimal::from_str("1200").unwrap());
    let pmt_monthly = Memo::new(move |_| {
        fin::calc_monthly_pmt(
            form_data.get().loan_amt,
            int_monthly.get(),
            form_data.get().num_pmts,
        )
    });
    let amort_sched = Memo::new(move |_| {
        fin::calc_amort_sched(
            form_data.get().loan_amt,
            int_monthly.get(),
            pmt_monthly.get(),
            form_data.get().num_pmts,
        )
    });
    let total_int = Memo::new(move |_| {
        pmt_monthly() * Decimal::from_str(&form_data().num_pmts.to_string()).unwrap()
            - form_data().loan_amt
    });

    let dates: Memo<Vec<NaiveDate>> = Memo::new(move |_| {
        date_util::get_date_vector(form_data.get().start_date, form_data.get().num_pmts)
    });

    view! {
        <div class="app">
            <Form
                set_form_data=set_form_data
                form_data=form_data
                set_url_data=set_url_data
                url_data=url_data
            />
            <p>
                Total interest paid over life of loan:
                <span class="highlight">
                    {move || format!("{:.2}", total_int()).separate_with_commas()}
                </span>
            </p>
            <AmortSchedTable rows=amort_sched dates=dates/>
        </div>
    }
}

#[component]
fn Form(
    set_form_data: WriteSignal<FormData>,
    form_data: ReadSignal<FormData>,
    set_url_data: SignalSetter<Option<String>>,
    url_data: Memo<Option<String>>,
) -> impl IntoView {
    const INT_YEARLY_PCT: &str = "int_yearly_pct";
    const LOAN_AMT: &str = "loan_amt";
    const NUM_PMTS: &str = "num_pmts";
    const TEXT: &str = "text";

    let mut loan_amt = form_data.get_untracked().loan_amt;
    let mut int_yearly_pct = form_data.get_untracked().int_yearly_pct;
    let mut num_pmts = form_data.get_untracked().num_pmts;
    let mut start_date = form_data.get_untracked().start_date;
    let input = url_data.get_untracked().unwrap_or_default();
    if let Some(fd) = decode_url_data(&input) {
        loan_amt = fd.loan_amt;
        int_yearly_pct = fd.int_yearly_pct;
        num_pmts = fd.num_pmts;
        start_date = fd.start_date;
    }

    view! {
        <form>
            <label for=LOAN_AMT>Loan Amount: </label>
            <input type=TEXT id=LOAN_AMT name=LOAN_AMT value={format!("{:.0}", loan_amt)} size="12"
                on:input=move |ev| {
                    let fd = FormData {
                        loan_amt: Decimal::from_str(&event_target_value(&ev)).unwrap(),
                        ..form_data.get_untracked()
                    };
                    set_form_data(fd);
                    set_url_data(Some(encode(&serde_json::to_string(&fd).unwrap()).into_owned()));
                }
            />
            <label for=INT_YEARLY_PCT>Annual Percentage Rate: </label>
            <input type=TEXT id=INT_YEARLY_PCT name=INT_YEARLY_PCT value={format!("{:.1}", int_yearly_pct)} size="4"
                on:input=move |ev| {
                    let fd = FormData {
                        int_yearly_pct: Decimal::from_str(&event_target_value(&ev)).unwrap(),
                        ..form_data.get_untracked()
                    };
                    set_form_data(fd);
                    set_url_data(Some(encode(&serde_json::to_string(&fd).unwrap()).into_owned()));
                }
            />
            <label for=NUM_PMTS>Number of Monthly Payments: </label>
            <input type=TEXT id=NUM_PMTS name=NUM_PMTS value={format!("{:.2}", num_pmts)} size="4" maxlength="3"
                on:input=move |ev| {
                    let fd = FormData {
                        num_pmts: event_target_value(&ev).parse::<u64>().unwrap(),
                        ..form_data.get_untracked()
                    };
                    set_form_data(fd);
                    set_url_data(Some(encode(&serde_json::to_string(&fd).unwrap()).into_owned()));
                }
            />
            <label for="start">Start date: </label>
            <input type="date" id="start" name="loan-start" value={format!("{}", start_date)}
                on:input=move |ev| {
                    let date_str = event_target_value(&ev);
                    if let Ok(start_date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        let fd = FormData {start_date, ..form_data.get_untracked()};
                        set_form_data(fd);
                        set_url_data(Some(encode(&serde_json::to_string(&fd).unwrap()).into_owned()));
                    }
                }
            />
        </form>
    }
}

#[component]
fn AmortSchedTable(rows: Memo<Vec<fin::AmortRow>>, dates: Memo<Vec<NaiveDate>>) -> impl IntoView {
    const COLS: [&str; 7] = [
        "Period",
        "Date",
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
       {move || rows().into_iter().map(
            |r| view! {
                <tr>
                    <td>{r.period}</td>
                    <td>{dates()[usize::try_from(r.period -1).unwrap()].to_string()}</td>
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
