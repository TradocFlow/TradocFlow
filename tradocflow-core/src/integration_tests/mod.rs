/// Integration tests module for Tradocflow Core
/// 
/// This module contains comprehensive integration tests that verify
/// the interaction between multiple components and services.

pub mod focus_management_integration_test;

pub use focus_management_integration_test::run_focus_management_integration_tests;

/// Run all integration tests
pub async fn run_all_integration_tests() -> anyhow::Result<()> {
    println!("🚀 Starting Tradocflow Core Integration Tests\n");
    
    // Run focus management integration tests
    run_focus_management_integration_tests().await?;
    
    // Add other integration test modules here as they are created
    
    println!("\n🎉 All integration tests completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn run_all_tests() {
        run_all_integration_tests().await.unwrap();
    }
}