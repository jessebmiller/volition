use rand::Rng;
use reqwest::Client;
use crate::api::chat_with_api;
use crate::utils::DebugLevel;
use crate::config::Config;
use crate::utils::git;
use crate::strategies::linear::linear_strategy;
use anyhow::Result;

/// Simulated Annealing Algorithm
/// This function performs simulated annealing to find an optimal solution using API calls.
///
/// # Arguments
/// * `client` - The HTTP client for making API requests.
/// * `config` - The configuration for API access.
/// * `user_goal` - The goal for the solution, included in commit messages.
/// * `max_iterations` - The maximum number of iterations to perform.
/// * `initial_temperature` - The starting temperature for the annealing process.
/// * `cooling_rate` - The rate at which the temperature decreases.
/// * `debug_level` - The level of debug information to log.
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
    debug_level: DebugLevel,
) -> Result<String> {
    // Create initial solution commit
    let mut current_solution = git::commit_current_state(
        &format!("Initial solution for goal: {}", user_goal),
        debug_level
    ).await?;

    let mut best_solution = current_solution.clone();
    git::tag_solution(&best_solution, "best_solution", debug_level).await?;

    let mut temperature = initial_temperature;

    for iteration in 0..max_iterations {
        // Generate and commit a neighboring solution
        git::checkout_solution(&current_solution, debug_level).await?;
        let neighbor_solution = generate_neighbor(client, config, user_goal, iteration, debug_level).await?;

        // Evaluate both solutions
        let neighbor_energy = evaluate_solution(client, config, &neighbor_solution, user_goal, debug_level).await?;
        let current_energy = evaluate_solution(client, config, &current_solution, user_goal, debug_level).await?;

        // Calculate the change in energy
        let delta_energy = neighbor_energy - current_energy;

        // Decide whether to accept the neighbor solution
        if delta_energy < 0.0 || rand::thread_rng().gen::<f64>() < (-(delta_energy / temperature)).exp() {
            current_solution = neighbor_solution.clone();
        }

        // Update the best solution found
        if neighbor_energy < evaluate_solution(client, config, &best_solution, user_goal, debug_level).await? {
            best_solution = neighbor_solution.clone();
            git::tag_solution(&best_solution, "best_solution", debug_level).await?;
        }

        // Decrease the temperature
        temperature *= 1.0 - cooling_rate;
    }

    // Return to the best solution before finishing
    git::checkout_solution(&best_solution, debug_level).await?;

    // Cleanup temporary branches and tags
    git::cleanup(debug_level).await?;

    Ok(best_solution)
}

/// Generate a neighboring solution using the linear strategy
async fn generate_neighbor(client: &Client, config: &Config, user_goal: &str, iteration: usize, debug_level: DebugLevel) -> Result<String> {
    // Use the linear strategy for generating neighbors
    linear_strategy(
        client,
        config,
        vec!["shell".to_string(), "read_file".to_string(), "write_file".to_string(), "search_code".to_string(), "find_definition".to_string(), "user_input".to_string()],
        user_goal,
        &format!("Generate a neighbor for goal: {}", user_goal),
        debug_level,
        vec![], // Provide an empty Vec<ResponseMessage> as the missing argument
    ).await?;

    let commit_message = format!("Neighbor solution for goal: {}, iteration: {}", user_goal, iteration);
    let commit_hash = git::commit_current_state(&commit_message, debug_level).await?;
    Ok(commit_hash)
}

/// Evaluate the energy of a solution using the API
async fn evaluate_solution(
    client: &Client,
    config: &Config,
    solution: &str,
    user_goal: &str,
    debug_level: DebugLevel,
) -> Result<f64> {
    // Checkout the solution to evaluate
    git::checkout_solution(solution, debug_level).await?;
    // Placeholder for calculating the energy based on the API response
    let response = chat_with_api(client, config, vec![], debug_level, None).await?;
    Ok(0.0)
}
