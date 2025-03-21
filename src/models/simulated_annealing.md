# Simulated Annealing Feature Documentation

## Overview

The simulated annealing feature is designed to optimize solutions using external models provided by APIs such as OpenAI or Ollama. This probabilistic technique approximates the global optimum of a given function by iteratively exploring neighboring solutions and accepting them based on a temperature schedule.

## Implementation Details

The feature is implemented in the `simulated_annealing.rs` file within the `src/models` directory. It integrates with the existing API functions to evaluate solutions and improve their quality.

### Key Components

- **Simulated Annealing Function**: The main function that performs the optimization process.
  - **Arguments**:
    - `client`: The HTTP client for making API requests.
    - `config`: The configuration for API access.
    - `initial_solution`: The starting point for the algorithm.
    - `max_iterations`: The maximum number of iterations to perform.
    - `initial_temperature`: The starting temperature for the annealing process.
    - `cooling_rate`: The rate at which the temperature decreases.
    - `debug_level`: The level of debug information to log.
  - **Returns**: The best solution found.

- **Neighbor Generation**: A placeholder function to generate neighboring solutions. This function should be customized to produce meaningful variations of the input messages.

- **Energy Evaluation**: The `evaluate_solution` function uses the API to assess the quality of solutions. The energy calculation is a placeholder and should be defined based on the desired optimization criteria.

## Integration with APIs

The algorithm interacts with the APIs using the `chat_with_api` function, which determines the appropriate service (OpenAI or Ollama) based on the configuration. It sends the current and neighboring solutions as messages to the API and uses the responses to guide the optimization process.

## Next Steps

1. **Customize Neighbor Generation**: Implement logic to generate meaningful neighboring solutions.
2. **Define Energy Calculation**: Determine how to calculate the energy or quality of solutions based on API responses.
3. **Testing**: Test the integration to ensure it improves the quality of solutions.

## Conclusion

The simulated annealing feature provides a flexible framework for optimizing solutions using external models. By customizing the neighbor generation and energy calculation, it can be tailored to achieve significant improvements in solution quality.