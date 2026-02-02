use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    pub construct_type: String,
    pub intent: String,
    pub assumptions_and_invariants: Vec<String>,
    pub common_failure_modes: Vec<String>,
    pub safest_edits: Vec<RankedEdit>,
    pub verify_commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RankedEdit {
    pub rank: u32,
    pub description: String,
}

pub async fn generate_explanation_from_llm(
    _code_snippet: &str,
    verify_commands: Vec<String>,
) -> Result<Explanation> {
    Ok(Explanation {
        construct_type: "Function Call".to_string(),
        intent:
            "This snippet appears to be calling a function to perform a small unit of behavior."
                .to_string(),
        assumptions_and_invariants: vec![
            "The function exists and accepts the provided arguments.".to_string(),
        ],
        common_failure_modes: vec![
            "TypeError if a referenced value is null/undefined.".to_string(),
        ],
        safest_edits: vec![RankedEdit {
            rank: 1,
            description: "Add a guard clause before the call when inputs can be missing."
                .to_string(),
        }],
        verify_commands,
    })
}
