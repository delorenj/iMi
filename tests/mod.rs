// Test module declarations for iMi test suite
// This file ensures that all test modules are properly declared and accessible

// Core test utilities - available to all other test modules
pub mod test_utilities;

// Test modules - each contains specific test categories
pub mod cli_init_integration;
pub mod comprehensive_init_tests;
pub mod comprehensive_test_runner;
pub mod database_tests;
pub mod docs_coverage_orchestrator;
pub mod docs_feat_rules_coverage;
pub mod docs_init_rules_coverage;
pub mod enhanced_config_tests;
pub mod enhanced_database_tests;
pub mod enhanced_error_tests;
pub mod enhanced_git_tests;
pub mod enhanced_init_tests;
pub mod error_scenario_comprehensive;
pub mod git_tests;
pub mod init_cli_behavior_tests;
pub mod init_cli_patch;
pub mod init_command_spec;
pub mod init_database_integration;
pub mod init_rules_tests;
pub mod init_tdd_comprehensive;
pub mod init_tests;
pub mod init_test_summary;
pub mod integration_tests_comprehensive;
pub mod monitor_tests;
pub mod path_construction_validation_tests;
pub mod property_based_tests;
pub mod test_architecture_master;
pub mod test_execution_framework;
pub mod unit_tests_comprehensive;
pub mod worktree_tests;

// Re-export commonly used test utilities for easier access
pub use test_utilities::{
    TestEnvironment, TestResult, TestDataBuilder,
    AssertionUtils, MockDataGenerator, PerformanceTestUtils,
    RepositoryBuilder, WorktreeBuilder, ActivityBuilder,
    BenchmarkResult
};