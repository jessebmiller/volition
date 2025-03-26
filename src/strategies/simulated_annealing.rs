use rand::Rng;
use reqwest::Client;
use log::info;
use crate::config::Config;
use crate::utils::git;
use crate::strategies::linear::linear_strategy;
use anyhow::Result;
use crate::models::chat::ResponseMessage;
use crate::constants::SYSTEM_PROMPT;
use crate::models::tools::SubmitQualityScoreArgs;
use crate::models::tools::Tools;

/// Simulated Annealing Algorithm
/// This function performs simulated annealing to find an optimal solution.
///
/// # Arguments
/// * `client` - The HTTP client for making API requests.
/// * `config` - The configuration for API access.
/// * `user_goal` - The goal for the solution, included in commit messages.
/// * `max_iterations` - The maximum number of iterations to perform.
/// * `initial_temperature` - The starting temperature for the annealing process.
/// * `cooling_rate` - The rate at which the temperature decreases.
///
/// # Returns
/// * The commit hash of the best solution found.
pub async fn simulated_annealing(
    client: &Client,
    config: &Config,
    user_goal: &str,
    max_iterations: usize,
    initial_temperature: f64,
    cooling_rate: f64,
    cleanup: bool, // Added parameter to control cleanup
) -> Result<String> {
    let repo_path = "."; // Assuming current directory is the repo path

    // Create initial solution commit
    let mut current_solution = git::commit_current_state(
        repo_path,
        &format!("Initial solution for goal: {}", user_goal),
    ).await?;

    let mut best_solution = current_solution.clone();
    git::tag_solution(repo_path, &best_solution, "best_solution").await?;

    let mut temperature = initial_temperature;

    for iteration in 0..max_iterations {
        // Create a new branch for this iteration with a meaningful name
        let branch_name = format!("solution-iteration-{}-{}", iteration, git::create_unique_branch_name());
        git::create_and_checkout_branch(repo_path, &branch_name).await?;

        // Generate and commit a neighboring solution
        git::checkout_solution(repo_path, &current_solution).await?;
        let neighbor_solution = generate_neighbor(client, config, user_goal, iteration).await?;

        // Evaluate both solutions
        let neighbor_score = evaluate_solution(client, config, &neighbor_solution, user_goal).await?;
        let neighbor_energy = 100.0 - neighbor_score;
        let current_energy = evaluate_solution(client, config, &current_solution, user_goal).await?;

        // Calculate the change in energy
        let delta_energy = neighbor_energy - current_energy;

        // Decide whether to accept the neighbor solution
        if delta_energy < 0.0 || rand::thread_rng().gen::<f64>() < (-(delta_energy / temperature)).exp() {
            current_solution = neighbor_solution.clone();
        }

        // Update the best solution found
        if neighbor_energy < evaluate_solution(client, config, &best_solution, user_goal).await? {
            best_solution = neighbor_solution.clone();
            git::tag_solution(repo_path, &best_solution, "best_solution").await?;
        }

        // Decrease the temperature
        temperature *= 1.0 - cooling_rate;
    }

    // Return to the best solution before finishing
    git::checkout_solution(repo_path, &best_solution).await?;

    // Optional cleanup of temporary branches and tags
    if cleanup {
        git::cleanup(repo_path).await?;
    }

    Ok(best_solution)
}

/// Generate a neighboring solution using the linear strategy
async fn generate_neighbor(client: &Client, config: &Config, user_goal: &str, iteration: usize) -> Result<String> {
    let repo_path = "."; // Assuming current directory is the repo path

    // Use the linear strategy for generating neighbors
    let system_prompt = SYSTEM_PROMPT;
    let messages = vec![
        ResponseMessage {
            role: "system".to_string(),
            content: Some(system_prompt.to_string()),
            tool_calls: None,
            tool_call_id: None,
        },
        ResponseMessage {
            role: "user".to_string(),
            content: Some(user_goal.to_string()),
            tool_calls: None,
            tool_call_id: None,
        }
    ];

    let _response_messages = linear_strategy( // Suppressed unused variable warning
        client,
        config,
        vec![
            Tools::shell_definition(),
            Tools::read_file_definition(),
            Tools::write_file_definition(),
            Tools::search_code_definition(),
            Tools::find_definition_definition(),
            Tools::user_input_definition(),
        ],
        "",
        messages,
    ).await?;

    let commit_message = format!("Neighbor solution for goal: {}, iteration: {}", user_goal, iteration);
    let commit_hash = git::commit_current_state(repo_path, &commit_message).await?;
    Ok(commit_hash)
}

/// Evaluate the energy of a solution using the API
async fn evaluate_solution(
    client: &Client,
    config: &Config,
    solution: &str,
    user_goal: &str,
) -> Result<f64> {
    let repo_path = "."; // Assuming current directory is the repo path

    // Checkout the solution to evaluate
    git::checkout_solution(repo_path, solution).await?;

    // System prompt for evaluation
    let system_prompt = "You are an AI model integrated into a simulated annealing algorithm, tasked with evaluating software solutions. Your responsibilities include:
    1. Context Understanding: Recognize that you are part of a simulated annealing process, where your evaluation helps in optimizing the solution iteratively.
    2. User Goal Evaluation: Assess how well the solution meets the specified user goal.
    3. Testing and Compilation: Run all available tests to ensure the solution's functionality and confirm that the project compiles without errors.
    4. Performance Metrics: Analyze the solution's performance based on predefined criteria, such as execution speed, resource utilization, and code quality.
    5. Energy Calculation: Provide a numerical score (energy value) representing the solution's quality. This score should be a floating-point number (f64), where lower scores indicate better solutions.
    6. Tool Utilization: You have access to tools that can assist in reading files, executing shell commands, and running tests. Use these tools to gather necessary information for your evaluation.
    7. Communication: Clearly communicate your findings and the calculated energy value in a structured format that can be easily parsed by the calling function.

    Rubric for Energy Calculation:
    - Functionality: Does the solution meet the user goal? (0-10)
    - Performance: Is the solution efficient enough for the situation? (0-10)
    - Code Quality: Is the code well-structured and maintainable? (0-10)
    - Testing: Do all tests pass successfully? (0-10)
    - Compilation: Does the solution compile without errors? (0-10)

    Total energy score = Sum of all criteria scores. Lower scores indicate better solutions.";

    // Use the linear strategy to evaluate the solution
    let messages = vec![
        ResponseMessage {
            role: "system".to_string(),
            content: Some(system_prompt.to_string()),
            tool_calls: None,
            tool_call_id: None,
        },
        ResponseMessage {
            role: "user".to_string(),
            content: Some(user_goal.to_string()),
            tool_calls: None,
            tool_call_id: None,
        }
    ];

    let response_messages = linear_strategy(
        client,
        config,
        vec![
            Tools::shell_definition(),
            Tools::read_file_definition(),
            Tools::write_file_definition(),
            Tools::search_code_definition(),
            Tools::find_definition_definition(),
            Tools::user_input_definition(),
            Tools::submit_quality_score_definition(),
        ],
        "submit_quality_score",
        messages,
    ).await?;

    // Extract the energy value from the response
    for message in response_messages {
        if let Some(tool_calls) = message.tool_calls {
            for tool_call in tool_calls {
                if tool_call.function.name == "submit_quality_score" {
                    let arguments = tool_call.function.arguments;
                    if let Ok(score_args) = serde_json::from_str::<SubmitQualityScoreArgs>(&arguments) {
                        info!("Extracted energy value: {}", score_args.score);
                        return Ok(score_args.score);
                    }
                }
            }
        }
    }

    // Return an error if the energy value could not be extracted
    Err(anyhow::anyhow!("Failed to extract energy value from response."))
}
