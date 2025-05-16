use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

pub async fn au_to_text(au: Arc<RwLock<Vec<f32>>>, lang: &str, path_to_model: &str) -> Result<String> {
    let au = au.read().await;
    let ctx = WhisperContext::new_with_params(
        path_to_model,
	WhisperContextParameters::default())?;
    let mut res = String::new();

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    let lang = lang.to_string().to_lowercase();
    params.set_language(Some(lang.as_str()));
    let mut state = ctx.create_state()?;

    let _r = state.full(params, au.as_slice())?;
    let num_segments = state.full_n_segments()?;
    for i in 0..num_segments {
        let segment = state.full_get_segment_text(i)?;
        res.push_str(segment.as_str());
    }

    Ok(res)
}


