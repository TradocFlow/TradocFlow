use std::collections::HashMap;

/// Test utilities for project creation wizard
pub struct ProjectWizardTestHelper {
    pub current_step: i32,
    pub project_name: String,
    pub selected_languages: Vec<String>,
    pub team_members: Vec<TestTeamMember>,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestTeamMember {
    pub name: String,
    pub email: String,
    pub role: String,
}

impl ProjectWizardTestHelper {
    pub fn new() -> Self {
        Self {
            current_step: 1,
            project_name: String::new(),
            selected_languages: Vec::new(),
            team_members: Vec::new(),
            validation_errors: Vec::new(),
        }
    }

    /// Test wizard navigation
    pub fn test_wizard_navigation(&mut self) -> Result<(), String> {
        // Test forward navigation
        for step in 1..=7 {
            self.current_step = step;
            if !self.can_navigate_to_step(step) {
                return Err(format!("Cannot navigate to step {}", step));
            }
        }

        // Test backward navigation
        for step in (1..7).rev() {
            self.current_step = step;
            if step > 1 && !self.can_go_back() {
                return Err(format!("Cannot go back from step {}", step));
            }
        }

        Ok(())
    }

    /// Test project details validation
    pub fn test_project_details_validation(&mut self) -> Result<(), String> {
        self.current_step = 1;
        self.validation_errors.clear();

        // Test empty name
        self.project_name = "".to_string();
        if !self.validate_project_details() {
            self.validation_errors.push("Project name is required".to_string());
        }

        // Test valid name
        self.project_name = "Test Project".to_string();
        if !self.validate_project_details() {
            return Err("Valid project name should pass validation".to_string());
        }

        Ok(())
    }

    /// Test language configuration validation
    pub fn test_language_configuration_validation(&mut self) -> Result<(), String> {
        self.current_step = 4;
        self.validation_errors.clear();

        // Test no target languages selected
        self.selected_languages.clear();
        if self.validate_language_configuration() {
            return Err("Should require at least one target language".to_string());
        }

        // Test valid configuration
        self.selected_languages = vec!["Spanish".to_string(), "French".to_string()];
        if !self.validate_language_configuration() {
            return Err("Valid language configuration should pass".to_string());
        }

        Ok(())
    }

    /// Test team member validation
    pub fn test_team_member_validation(&mut self) -> Result<(), String> {
        self.current_step = 5;
        self.validation_errors.clear();

        // Test invalid email
        let invalid_member = TestTeamMember {
            name: "John Doe".to_string(),
            email: "invalid-email".to_string(),
            role: "Translator".to_string(),
        };

        if self.validate_team_member(&invalid_member) {
            return Err("Should reject invalid email format".to_string());
        }

        // Test valid member
        let valid_member = TestTeamMember {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            role: "Translator".to_string(),
        };

        if !self.validate_team_member(&valid_member) {
            return Err("Should accept valid team member".to_string());
        }

        Ok(())
    }

    /// Test complete wizard workflow
    pub fn test_complete_workflow(&mut self) -> Result<(), String> {
        // Step 1: Project Details
        self.current_step = 1;
        self.project_name = "Complete Test Project".to_string();
        if !self.validate_current_step() {
            return Err("Step 1 validation failed".to_string());
        }

        // Step 2: Template Selection (always valid)
        self.current_step = 2;
        if !self.validate_current_step() {
            return Err("Step 2 validation failed".to_string());
        }

        // Step 3: Project Location (assume valid path)
        self.current_step = 3;
        if !self.validate_current_step() {
            return Err("Step 3 validation failed".to_string());
        }

        // Step 4: Language Configuration
        self.current_step = 4;
        self.selected_languages = vec!["Spanish".to_string(), "German".to_string()];
        if !self.validate_current_step() {
            return Err("Step 4 validation failed".to_string());
        }

        // Step 5: Team Setup
        self.current_step = 5;
        self.team_members = vec![
            TestTeamMember {
                name: "Alice Translator".to_string(),
                email: "alice@example.com".to_string(),
                role: "Translator".to_string(),
            },
            TestTeamMember {
                name: "Bob Reviewer".to_string(),
                email: "bob@example.com".to_string(),
                role: "Reviewer".to_string(),
            },
        ];
        if !self.validate_current_step() {
            return Err("Step 5 validation failed".to_string());
        }

        // Step 6: Project Structure (always valid)
        self.current_step = 6;
        if !self.validate_current_step() {
            return Err("Step 6 validation failed".to_string());
        }

        // Step 7: Review (always valid if previous steps passed)
        self.current_step = 7;
        if !self.validate_current_step() {
            return Err("Step 7 validation failed".to_string());
        }

        Ok(())
    }

    // Helper methods for validation logic

    fn can_navigate_to_step(&self, step: i32) -> bool {
        step >= 1 && step <= 7
    }

    fn can_go_back(&self) -> bool {
        self.current_step > 1
    }

    fn validate_project_details(&self) -> bool {
        !self.project_name.trim().is_empty()
    }

    fn validate_language_configuration(&self) -> bool {
        !self.selected_languages.is_empty()
    }

    fn validate_team_member(&self, member: &TestTeamMember) -> bool {
        !member.name.trim().is_empty() 
            && member.email.contains('@')
            && !member.role.trim().is_empty()
    }

    fn validate_current_step(&self) -> bool {
        match self.current_step {
            1 => self.validate_project_details(),
            2 => true, // Template selection is always valid
            3 => true, // Assume folder path is valid for testing
            4 => self.validate_language_configuration(),
            5 => self.team_members.iter().all(|m| self.validate_team_member(m)),
            6 => true, // Project structure is always valid
            7 => true, // Review is always valid if we got here
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_navigation() {
        let mut helper = ProjectWizardTestHelper::new();
        assert!(helper.test_wizard_navigation().is_ok());
    }

    #[test]
    fn test_project_validation() {
        let mut helper = ProjectWizardTestHelper::new();
        assert!(helper.test_project_details_validation().is_ok());
    }

    #[test]
    fn test_language_validation() {
        let mut helper = ProjectWizardTestHelper::new();
        assert!(helper.test_language_configuration_validation().is_ok());
    }

    #[test]
    fn test_team_member_validation() {
        let mut helper = ProjectWizardTestHelper::new();
        assert!(helper.test_team_member_validation().is_ok());
    }

    #[test]
    fn test_complete_workflow() {
        let mut helper = ProjectWizardTestHelper::new();
        assert!(helper.test_complete_workflow().is_ok());
    }

    #[test]
    fn test_validation_error_handling() {
        let mut helper = ProjectWizardTestHelper::new();
        
        // Test that empty project name fails validation
        helper.project_name = "".to_string();
        helper.current_step = 1;
        assert!(!helper.validate_current_step());

        // Test that no target languages fails validation
        helper.selected_languages.clear();
        helper.current_step = 4;
        assert!(!helper.validate_current_step());

        // Test that invalid email fails validation
        let invalid_member = TestTeamMember {
            name: "Test User".to_string(),
            email: "invalid".to_string(),
            role: "Translator".to_string(),
        };
        assert!(!helper.validate_team_member(&invalid_member));
    }

    #[test]
    fn test_step_progression() {
        let mut helper = ProjectWizardTestHelper::new();
        
        // Should start at step 1
        assert_eq!(helper.current_step, 1);
        
        // Should be able to navigate through all steps
        for step in 1..=7 {
            helper.current_step = step;
            assert!(helper.can_navigate_to_step(step));
        }
        
        // Should not be able to go back from step 1
        helper.current_step = 1;
        assert!(!helper.can_go_back());
        
        // Should be able to go back from other steps
        for step in 2..=7 {
            helper.current_step = step;
            assert!(helper.can_go_back());
        }
    }
}