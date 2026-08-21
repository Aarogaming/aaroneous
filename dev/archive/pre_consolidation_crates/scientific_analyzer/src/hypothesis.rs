// HYPOTHESIS Phase: Generate testable hypotheses from AST observations
// Uses compute::bayesian for prior confidence estimation

use crate::ast_parser::{AstObservation, CodeStructure, StructureType};
use serde::{Deserialize, Serialize};

/// A testable hypothesis about code behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub target: String,
    pub observation: String,
    pub prediction: String,
    pub prior_confidence: f64,
    pub risk_factors: Vec<String>,
    pub experiment_design: ExperimentDesign,
}

/// Design for testing a hypothesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentDesign {
    pub test_type: TestType,
    pub input_data: Vec<String>,
    pub expected_behavior: String,
    pub failure_conditions: Vec<String>,
    pub performance_threshold: Option<f64>, // Max acceptable ms
    pub iterations: usize,
}

/// Type of test to run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestType {
    UnitTest,
    Benchmark,
    IntegrationTest,
    StressTest,
}

impl Hypothesis {
    /// Create a new hypothesis with minimal parameters.
    pub fn new(target: &str, observation: &str, prior_confidence: f64) -> Self {
        Self {
            target: target.to_string(),
            observation: observation.to_string(),
            prediction: String::new(),
            prior_confidence,
            risk_factors: vec![],
            experiment_design: ExperimentDesign {
                test_type: TestType::UnitTest,
                input_data: vec![],
                expected_behavior: String::new(),
                failure_conditions: vec![],
                performance_threshold: None,
                iterations: 1,
            },
        }
    }

    /// Generate hypotheses from an AST observation
    pub fn from_observation(observation: &AstObservation) -> Vec<Self> {
        let mut hypotheses = Vec::new();

        for structure in &observation.structures {
            match structure.structure_type {
                StructureType::Function | StructureType::Method => {
                    hypotheses.push(Self::generate_function_hypothesis(observation, structure));
                }
                _ => {}
            }
        }

        hypotheses
    }

    /// Generate a hypothesis for a single function
    fn generate_function_hypothesis(
        observation: &AstObservation,
        structure: &CodeStructure,
    ) -> Self {
        // Observation summary
        let observation_text = format!(
            "Function `{}` with {} parameters, returns {:?}, complexity: {}",
            structure.signature.name,
            structure.signature.parameters.len(),
            structure.signature.return_type,
            structure.control_flow_complexity,
        );

        // Generate prediction based on structure
        let prediction = Self::generate_prediction(structure);

        // Calculate prior confidence using Bayesian estimation
        let prior_confidence = Self::calculate_prior_confidence(structure, observation);

        // Identify risk factors
        let risk_factors = Self::identify_risks(structure);

        // Design experiment
        let experiment_design = Self::design_experiment(structure);

        Self {
            target: format!("{}::{}", observation.file_path, structure.signature.name),
            observation: observation_text,
            prediction,
            prior_confidence,
            risk_factors,
            experiment_design,
        }
    }

    /// Generate a prediction about function behavior
    fn generate_prediction(structure: &CodeStructure) -> String {
        let param_count = structure.signature.parameters.len();
        let has_return = structure.signature.return_type.is_some();

        if param_count == 0 && has_return {
            format!(
                "`{}` should return a value without side effects",
                structure.signature.name
            )
        } else if param_count > 0 && has_return {
            format!(
                "`{}` should transform {} input(s) into output",
                structure.signature.name, param_count
            )
        } else {
            format!(
                "`{}` should execute with side effects",
                structure.signature.name
            )
        }
    }

    /// Calculate prior confidence using Bayesian estimation
    fn calculate_prior_confidence(structure: &CodeStructure, observation: &AstObservation) -> f64 {
        // Prior based on code complexity and structure
        let complexity_penalty = (structure.control_flow_complexity as f64 * 0.1).min(0.5);
        let entropy_penalty = (observation.raw_entropy / 8.0).min(0.3);

        // Base confidence
        let base_confidence = 0.8;

        // Apply penalties
        let confidence = base_confidence - complexity_penalty - entropy_penalty;

        confidence.clamp(0.1, 0.95)
    }

    /// Identify risk factors in the code structure
    fn identify_risks(structure: &CodeStructure) -> Vec<String> {
        let mut risks = Vec::new();

        if structure.control_flow_complexity > 5 {
            risks.push("High cyclomatic complexity".to_string());
        }

        if structure.signature.parameters.len() > 5 {
            risks.push("Many parameters (potential coupling)".to_string());
        }

        if let Some(ref return_type) = structure.signature.return_type {
            if return_type.contains("Result") || return_type.contains("Option") {
                risks.push("Fallible operation".to_string());
            }
        }

        if structure.line_range.1 - structure.line_range.0 > 100 {
            risks.push("Long function body".to_string());
        }

        risks
    }

    /// Design an experiment to test the hypothesis
    fn design_experiment(structure: &CodeStructure) -> ExperimentDesign {
        let param_count = structure.signature.parameters.len();

        let test_type = if param_count <= 2 {
            TestType::UnitTest
        } else if structure.line_range.1 - structure.line_range.0 > 50 {
            TestType::Benchmark
        } else {
            TestType::IntegrationTest
        };

        // Generate test inputs based on parameter types
        let input_data = Self::generate_test_inputs(structure);

        // Expected behavior
        let expected_behavior =
            "Function should complete without panic and return valid output".to_string();

        // Failure conditions
        let failure_conditions = vec![
            "Panic during execution".to_string(),
            "Infinite loop detected (timeout)".to_string(),
            "Memory allocation failure".to_string(),
        ];

        // Performance threshold (generous default)
        let performance_threshold = Some(1000.0); // 1 second

        ExperimentDesign {
            test_type,
            input_data,
            expected_behavior,
            failure_conditions,
            performance_threshold,
            iterations: 100,
        }
    }

    /// Generate test inputs based on parameter types
    fn generate_test_inputs(structure: &CodeStructure) -> Vec<String> {
        let mut inputs = Vec::new();

        for param in &structure.signature.parameters {
            let input = match param.param_type.as_str() {
                t if t.contains("i32")
                    || t.contains("i64")
                    || t.contains("u32")
                    || t.contains("u64") =>
                {
                    "42".to_string()
                }
                t if t.contains("f32") || t.contains("f64") => "3.14".to_string(),
                t if t.contains("String") || t.contains("str") => "\"test_input\"".to_string(),
                t if t.contains("Vec") => "vec![1, 2, 3]".to_string(),
                t if t.contains("bool") => "true".to_string(),
                _ => {
                    format!("/* {} */", param.param_type)
                }
            };
            inputs.push(input);
        }

        inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_parser::{
        AstObservation, CodeStructure, FunctionSignature, Parameter, StructureType, Visibility,
    };

    #[test]
    fn test_hypothesis_generation() {
        let structure = CodeStructure {
            name: "test_fn".to_string(),
            structure_type: StructureType::Function,
            signature: FunctionSignature {
                name: "test_fn".to_string(),
                parameters: vec![
                    Parameter {
                        name: "x".to_string(),
                        param_type: "i32".to_string(),
                    },
                    Parameter {
                        name: "y".to_string(),
                        param_type: "String".to_string(),
                    },
                ],
                return_type: Some("Result<(), Error>".to_string()),
                is_async: false,
                visibility: Visibility::Public,
            },
            line_range: (1, 20),
            dependencies: Vec::new(),
            control_flow_complexity: 3,
        };

        let observation = AstObservation {
            file_path: "test.rs".to_string(),
            language: crate::ast_parser::Language::Rust,
            structures: vec![structure.clone()],
            complexity_metrics: crate::ast_parser::ComplexityMetrics {
                cyclomatic_complexity: 3,
                lines_of_code: 20,
                nesting_depth: 2,
                branch_count: 2,
                call_count: 5,
            },
            raw_entropy: 4.5,
        };

        let hypotheses = Hypothesis::from_observation(&observation);
        assert_eq!(hypotheses.len(), 1);
        assert!(hypotheses[0].prior_confidence > 0.0 && hypotheses[0].prior_confidence < 1.0);
        assert!(!hypotheses[0].risk_factors.is_empty());
    }

    #[test]
    fn test_experiment_design() {
        let structure = CodeStructure {
            name: "simple_fn".to_string(),
            structure_type: StructureType::Function,
            signature: FunctionSignature {
                name: "simple_fn".to_string(),
                parameters: vec![Parameter {
                    name: "x".to_string(),
                    param_type: "i32".to_string(),
                }],
                return_type: Some("i32".to_string()),
                is_async: false,
                visibility: Visibility::Public,
            },
            line_range: (1, 10),
            dependencies: Vec::new(),
            control_flow_complexity: 1,
        };

        let design = Hypothesis::design_experiment(&structure);
        assert_eq!(design.input_data.len(), 1);
        assert!(design.iterations > 0);
    }
}
