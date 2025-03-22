use crate::models::tools::SubmitQualityScoreArgs;
use anyhow::Result;

/// Submits the quality score for a solution.
///
/// # Arguments
/// * `score` - The quality score to submit.
/// * `reason` - The reason for the given score.
///
/// # Returns
/// * A result indicating success or failure.
pub async fn submit_quality_score(args: SubmitQualityScoreArgs) -> Result<()> {
    // we just print it to the console to show the user the
    // reason. There is no result needed to send back to the AI
    println!("Submitting quality score: {} with reason: {}", args.score, args.reason);
    Ok(())
}
