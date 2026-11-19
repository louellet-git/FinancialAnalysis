//! Example: run a DCF verdict at the model's rf and at today's Treasury rf, and show
//! how much of the rate is real rate and how much is expected inflation.
//!
//! Run from rust/:  cargo run --example dcf_rates -- IBM 231.09
use financial_analysis::{Fa, VerdictOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let ticker = args.next().unwrap_or_else(|| "IBM".into());
    let price: f64 = args.next().as_deref().unwrap_or("231.09").parse()?;
    let fa = Fa::from_env();

    let model = fa.verdict(&ticker, price, &VerdictOptions::default())?;
    let today = fa.verdict(
        &ticker,
        price,
        &VerdictOptions { rf_date: Some("today".into()), ..Default::default() },
    )?;
    let r = today.rates.as_ref().expect("rates are present when rf_date is set");
    let pct = |x: f64| x * 100.0;

    println!("{ticker} at ${price:.2}");
    println!(
        "  rf {:.2}% on {} = real {:.2}% + expected inflation {:.2}% (model rf {:.2}%)",
        pct(r.rf.value), r.rf.date, pct(r.real_10y), pct(r.breakeven_10y), pct(r.model_rf_mean)
    );
    println!(
        "  5y5y forward inflation {:.2}%; CPI {:.2}% for {}{}",
        pct(r.forward_5y5y),
        pct(r.cpi_12m.value),
        &r.cpi_12m.date[..7],
        if r.cpi_12m.note.is_empty() { String::new() } else { format!(" ({})", r.cpi_12m.note) }
    );
    println!(
        "  terminal growth {:.2}% = 5y5y {:.2}% + real {:.2}%",
        pct(r.terminal_growth_mean), pct(r.forward_5y5y), pct(r.terminal_growth_real_vs_5y5y)
    );
    println!(
        "  P50 ${:.2} at model rf -> ${:.2} at Treasury rf ({:+.1}%)",
        model.p50, today.p50, (today.p50 / model.p50 - 1.0) * 100.0
    );
    println!(
        "  chance the price is too high {:.1}% -> {:.1}%: {} -> {}",
        pct(model.chance_price_too_high), pct(today.chance_price_too_high), model.verdict, today.verdict
    );
    Ok(())
}
