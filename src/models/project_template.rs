use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Project template for different types of documentation projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: TemplateCategory,
    pub preview_content: String,
    pub recommended_languages: Vec<String>,
    pub use_cases: Vec<String>,
    pub initial_chapters: Vec<ChapterTemplate>,
    pub default_settings: ProjectTemplateSettings,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TemplateCategory {
    #[serde(rename = "technical")]
    Technical,
    #[serde(rename = "user_guide")]
    UserGuide,
    #[serde(rename = "training")]
    Training,
    #[serde(rename = "policy")]
    Policy,
    #[serde(rename = "marketing")]
    Marketing,
    #[serde(rename = "blank")]
    Blank,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterTemplate {
    pub chapter_number: u32,
    pub title: String,
    pub slug: String,
    pub content_template: String,
    pub is_required: bool,
    pub estimated_words: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplateSettings {
    pub document_naming_convention: String, // "{number}_{slug}.md"
    pub default_export_formats: Vec<String>, // ["markdown", "html", "pdf"]
    pub enable_git_integration: bool,
    pub default_workflow: String, // "draft -> review -> approve -> publish"
    pub rtl_support: bool,
    pub font_preferences: HashMap<String, String>, // language -> font
}

/// Language configuration for project creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    pub code: String,
    pub name: String,
    pub is_rtl: bool,
    pub font_family: Option<String>,
    pub enabled: bool,
}

/// Project creation wizard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWizardData {
    // Step 1: Project Details
    pub name: String,
    pub description: Option<String>,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    pub priority: crate::models::project::Priority,
    
    // Step 2: Template Selection
    pub template_id: String,
    pub custom_chapters: Vec<ChapterTemplate>,
    
    // Step 3: Language Configuration
    pub source_language: String,
    pub target_languages: Vec<LanguageConfig>,
    
    // Step 4: Team Setup
    pub team_members: Vec<TeamMemberConfig>,
    pub project_roles: HashMap<String, Vec<String>>, // role -> permissions
    
    // Step 5: Project Structure
    pub chapter_organization: ChapterOrganization,
    pub initial_content: HashMap<String, String>, // chapter_slug -> content
    
    // Step 6: Advanced Configuration
    pub export_formats: Vec<String>,
    pub git_integration: GitIntegrationConfig,
    pub workflow_settings: WorkflowSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberConfig {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: String, // "owner", "translator", "reviewer", "viewer"
    pub languages: Vec<String>, // languages this member can work with
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterOrganization {
    pub structure_type: String, // "linear", "hierarchical", "modular"
    pub numbering_style: String, // "numeric", "alphabetic", "custom"
    pub chapter_prefix: Option<String>,
    pub auto_generate_toc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitIntegrationConfig {
    pub enabled: bool,
    pub repository_url: Option<String>,
    pub branch_strategy: String, // "feature", "language", "chapter"
    pub commit_message_template: String,
    pub auto_commit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSettings {
    pub workflow_type: String, // "simple", "review", "approval", "custom"
    pub states: Vec<String>, // ["draft", "review", "approved", "published"]
    pub auto_transitions: HashMap<String, String>, // from_state -> to_state
    pub required_reviewers: u32,
}

/// Template manager for handling project templates
pub struct TemplateManager {
    templates: Vec<ProjectTemplate>,
}

impl TemplateManager {
    pub fn new() -> Self {
        Self {
            templates: Self::create_default_templates(),
        }
    }
    
    pub fn get_all_templates(&self) -> &Vec<ProjectTemplate> {
        &self.templates
    }
    
    pub fn get_template(&self, id: &str) -> Option<&ProjectTemplate> {
        self.templates.iter().find(|t| t.id == id)
    }
    
    pub fn get_templates_by_category(&self, category: &TemplateCategory) -> Vec<&ProjectTemplate> {
        self.templates.iter().filter(|t| &t.category == category).collect()
    }
    
    fn create_default_templates() -> Vec<ProjectTemplate> {
        vec![
            // Technical Manual Template
            ProjectTemplate {
                id: "technical_manual".to_string(),
                name: "Technical Manual".to_string(),
                description: "Comprehensive technical documentation with installation, configuration, and troubleshooting guides".to_string(),
                category: TemplateCategory::Technical,
                preview_content: "# Technical Manual\n\nComplete documentation for technical systems including installation guides, API references, and troubleshooting sections.".to_string(),
                recommended_languages: vec!["en".to_string(), "de".to_string(), "fr".to_string(), "es".to_string()],
                use_cases: vec![
                    "Software documentation".to_string(),
                    "API documentation".to_string(),
                    "System administration guides".to_string(),
                    "Developer onboarding".to_string(),
                ],
                initial_chapters: vec![
                    ChapterTemplate {
                        chapter_number: 1,
                        title: "Introduction".to_string(),
                        slug: "introduction".to_string(),
                        content_template: "# Introduction\n\n## Overview\n\nProvide a high-level overview of the system.\n\n## Prerequisites\n\nList the prerequisites for using this system.\n\n## Document Structure\n\nExplain how this documentation is organized.".to_string(),
                        is_required: true,
                        estimated_words: Some(300),
                    },
                    ChapterTemplate {
                        chapter_number: 2,
                        title: "Installation Guide".to_string(),
                        slug: "installation".to_string(),
                        content_template: "# Installation Guide\n\n## System Requirements\n\n## Installation Steps\n\n### Quick Install\n\n### Custom Installation\n\n## Verification\n\n## Troubleshooting".to_string(),
                        is_required: true,
                        estimated_words: Some(800),
                    },
                    ChapterTemplate {
                        chapter_number: 3,
                        title: "Configuration".to_string(),
                        slug: "configuration".to_string(),
                        content_template: "# Configuration\n\n## Basic Configuration\n\n## Advanced Settings\n\n## Environment Variables\n\n## Configuration Files".to_string(),
                        is_required: true,
                        estimated_words: Some(600),
                    },
                    ChapterTemplate {
                        chapter_number: 4,
                        title: "User Guide".to_string(),
                        slug: "user-guide".to_string(),
                        content_template: "# User Guide\n\n## Getting Started\n\n## Basic Operations\n\n## Advanced Features\n\n## Best Practices".to_string(),
                        is_required: false,
                        estimated_words: Some(1200),
                    },
                    ChapterTemplate {
                        chapter_number: 5,
                        title: "API Reference".to_string(),
                        slug: "api-reference".to_string(),
                        content_template: "# API Reference\n\n## Authentication\n\n## Endpoints\n\n## Response Formats\n\n## Error Codes\n\n## Examples".to_string(),
                        is_required: false,
                        estimated_words: Some(1500),
                    },
                    ChapterTemplate {
                        chapter_number: 6,
                        title: "Troubleshooting".to_string(),
                        slug: "troubleshooting".to_string(),
                        content_template: "# Troubleshooting\n\n## Common Issues\n\n## Error Messages\n\n## Diagnostic Tools\n\n## Getting Help".to_string(),
                        is_required: true,
                        estimated_words: Some(500),
                    },
                ],
                default_settings: ProjectTemplateSettings {
                    document_naming_convention: "{number:02}_{slug}.md".to_string(),
                    default_export_formats: vec!["markdown".to_string(), "html".to_string(), "pdf".to_string()],
                    enable_git_integration: true,
                    default_workflow: "draft -> review -> approved -> published".to_string(),
                    rtl_support: false,
                    font_preferences: HashMap::from([
                        ("en".to_string(), "Liberation Sans".to_string()),
                        ("de".to_string(), "Liberation Sans".to_string()),
                    ]),
                },
                metadata: HashMap::from([
                    ("complexity".to_string(), "high".to_string()),
                    ("estimated_duration".to_string(), "2-4 weeks".to_string()),
                    ("team_size".to_string(), "3-5 people".to_string()),
                ]),
            },
            
            // User Guide Template
            ProjectTemplate {
                id: "user_guide".to_string(),
                name: "User Guide".to_string(),
                description: "End-user focused documentation with step-by-step instructions and tutorials".to_string(),
                category: TemplateCategory::UserGuide,
                preview_content: "# User Guide\n\nFriendly, accessible documentation designed for end users with clear instructions and helpful examples.".to_string(),
                recommended_languages: vec!["en".to_string(), "es".to_string(), "fr".to_string(), "de".to_string(), "it".to_string()],
                use_cases: vec![
                    "Product documentation".to_string(),
                    "Software tutorials".to_string(),
                    "Customer support guides".to_string(),
                    "Training materials".to_string(),
                ],
                initial_chapters: vec![
                    ChapterTemplate {
                        chapter_number: 1,
                        title: "Welcome".to_string(),
                        slug: "welcome".to_string(),
                        content_template: "# Welcome\n\n## What You'll Learn\n\n## How to Use This Guide\n\n## Getting Help".to_string(),
                        is_required: true,
                        estimated_words: Some(200),
                    },
                    ChapterTemplate {
                        chapter_number: 2,
                        title: "Quick Start".to_string(),
                        slug: "quick-start".to_string(),
                        content_template: "# Quick Start\n\n## First Steps\n\n## Basic Tasks\n\n## Your First Success".to_string(),
                        is_required: true,
                        estimated_words: Some(400),
                    },
                    ChapterTemplate {
                        chapter_number: 3,
                        title: "Step-by-Step Tutorial".to_string(),
                        slug: "tutorial".to_string(),
                        content_template: "# Step-by-Step Tutorial\n\n## Tutorial Overview\n\n## Step 1: Getting Started\n\n## Step 2: Basic Operations\n\n## Step 3: Advanced Features".to_string(),
                        is_required: false,
                        estimated_words: Some(1000),
                    },
                    ChapterTemplate {
                        chapter_number: 4,
                        title: "Frequently Asked Questions".to_string(),
                        slug: "faq".to_string(),
                        content_template: "# Frequently Asked Questions\n\n## General Questions\n\n## Technical Questions\n\n## Troubleshooting".to_string(),
                        is_required: false,
                        estimated_words: Some(600),
                    },
                ],
                default_settings: ProjectTemplateSettings {
                    document_naming_convention: "{number:02}_{slug}.md".to_string(),
                    default_export_formats: vec!["html".to_string(), "pdf".to_string()],
                    enable_git_integration: false,
                    default_workflow: "draft -> review -> published".to_string(),
                    rtl_support: false,
                    font_preferences: HashMap::new(),
                },
                metadata: HashMap::from([
                    ("complexity".to_string(), "medium".to_string()),
                    ("estimated_duration".to_string(), "1-2 weeks".to_string()),
                    ("team_size".to_string(), "2-3 people".to_string()),
                ]),
            },
            
            // Training Material Template
            ProjectTemplate {
                id: "training_material".to_string(),
                name: "Training Material".to_string(),
                description: "Educational content with lessons, exercises, and assessments".to_string(),
                category: TemplateCategory::Training,
                preview_content: "# Training Material\n\nStructured learning content with lessons, practical exercises, and knowledge assessments.".to_string(),
                recommended_languages: vec!["en".to_string(), "es".to_string(), "fr".to_string()],
                use_cases: vec![
                    "Employee training".to_string(),
                    "Product training".to_string(),
                    "Compliance training".to_string(),
                    "Skills development".to_string(),
                ],
                initial_chapters: vec![
                    ChapterTemplate {
                        chapter_number: 1,
                        title: "Course Introduction".to_string(),
                        slug: "introduction".to_string(),
                        content_template: "# Course Introduction\n\n## Learning Objectives\n\n## Course Structure\n\n## Prerequisites\n\n## Assessment Methods".to_string(),
                        is_required: true,
                        estimated_words: Some(300),
                    },
                    ChapterTemplate {
                        chapter_number: 2,
                        title: "Module 1: Fundamentals".to_string(),
                        slug: "fundamentals".to_string(),
                        content_template: "# Module 1: Fundamentals\n\n## Key Concepts\n\n## Practical Examples\n\n## Hands-on Exercise\n\n## Knowledge Check".to_string(),
                        is_required: true,
                        estimated_words: Some(800),
                    },
                    ChapterTemplate {
                        chapter_number: 3,
                        title: "Module 2: Intermediate Topics".to_string(),
                        slug: "intermediate".to_string(),
                        content_template: "# Module 2: Intermediate Topics\n\n## Advanced Concepts\n\n## Case Studies\n\n## Practice Activities\n\n## Quiz".to_string(),
                        is_required: false,
                        estimated_words: Some(1000),
                    },
                    ChapterTemplate {
                        chapter_number: 4,
                        title: "Final Assessment".to_string(),
                        slug: "assessment".to_string(),
                        content_template: "# Final Assessment\n\n## Assessment Instructions\n\n## Evaluation Criteria\n\n## Resources\n\n## Next Steps".to_string(),
                        is_required: true,
                        estimated_words: Some(400),
                    },
                ],
                default_settings: ProjectTemplateSettings {
                    document_naming_convention: "module_{number}_{slug}.md".to_string(),
                    default_export_formats: vec!["html".to_string(), "pdf".to_string()],
                    enable_git_integration: false,
                    default_workflow: "draft -> review -> approved -> published".to_string(),
                    rtl_support: false,
                    font_preferences: HashMap::new(),
                },
                metadata: HashMap::from([
                    ("complexity".to_string(), "medium".to_string()),
                    ("estimated_duration".to_string(), "2-3 weeks".to_string()),
                    ("team_size".to_string(), "2-4 people".to_string()),
                ]),
            },
            
            // Blank Template
            ProjectTemplate {
                id: "blank".to_string(),
                name: "Blank Project".to_string(),
                description: "Start with a clean slate and build your documentation structure from scratch".to_string(),
                category: TemplateCategory::Blank,
                preview_content: "# New Project\n\nStart building your documentation with complete flexibility and customization.".to_string(),
                recommended_languages: vec!["en".to_string()],
                use_cases: vec![
                    "Custom documentation".to_string(),
                    "Unique project requirements".to_string(),
                    "Experimental formats".to_string(),
                ],
                initial_chapters: vec![
                    ChapterTemplate {
                        chapter_number: 1,
                        title: "Getting Started".to_string(),
                        slug: "getting-started".to_string(),
                        content_template: "# Getting Started\n\nWelcome to your new project! Start writing your content here.".to_string(),
                        is_required: false,
                        estimated_words: Some(100),
                    },
                ],
                default_settings: ProjectTemplateSettings {
                    document_naming_convention: "{number:02}_{slug}.md".to_string(),
                    default_export_formats: vec!["markdown".to_string()],
                    enable_git_integration: false,
                    default_workflow: "draft -> published".to_string(),
                    rtl_support: false,
                    font_preferences: HashMap::new(),
                },
                metadata: HashMap::from([
                    ("complexity".to_string(), "low".to_string()),
                    ("estimated_duration".to_string(), "flexible".to_string()),
                    ("team_size".to_string(), "any".to_string()),
                ]),
            },
        ]
    }
}

impl Default for ProjectWizardData {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            due_date: None,
            priority: crate::models::project::Priority::Medium,
            template_id: "blank".to_string(),
            custom_chapters: Vec::new(),
            source_language: "en".to_string(),
            target_languages: Vec::new(),
            team_members: Vec::new(),
            project_roles: HashMap::new(),
            chapter_organization: ChapterOrganization {
                structure_type: "linear".to_string(),
                numbering_style: "numeric".to_string(),
                chapter_prefix: None,
                auto_generate_toc: true,
            },
            initial_content: HashMap::new(),
            export_formats: vec!["markdown".to_string()],
            git_integration: GitIntegrationConfig {
                enabled: false,
                repository_url: None,
                branch_strategy: "feature".to_string(),
                commit_message_template: "docs: {action} {chapter}".to_string(),
                auto_commit: false,
            },
            workflow_settings: WorkflowSettings {
                workflow_type: "simple".to_string(),
                states: vec!["draft".to_string(), "published".to_string()],
                auto_transitions: HashMap::new(),
                required_reviewers: 0,
            },
        }
    }
}

/// Available languages with display names and RTL support
pub fn get_available_languages() -> Vec<LanguageConfig> {
    vec![
        LanguageConfig {
            code: "en".to_string(),
            name: "English".to_string(),
            is_rtl: false,
            font_family: Some("Liberation Sans".to_string()),
            enabled: true,
        },
        LanguageConfig {
            code: "es".to_string(),
            name: "Spanish (Español)".to_string(),
            is_rtl: false,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
        LanguageConfig {
            code: "fr".to_string(),
            name: "French (Français)".to_string(),
            is_rtl: false,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
        LanguageConfig {
            code: "de".to_string(),
            name: "German (Deutsch)".to_string(),
            is_rtl: false,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
        LanguageConfig {
            code: "it".to_string(),
            name: "Italian (Italiano)".to_string(),
            is_rtl: false,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
        LanguageConfig {
            code: "nl".to_string(),
            name: "Dutch (Nederlands)".to_string(),
            is_rtl: false,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
        LanguageConfig {
            code: "pt".to_string(),
            name: "Portuguese (Português)".to_string(),
            is_rtl: false,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
        LanguageConfig {
            code: "ru".to_string(),
            name: "Russian (Русский)".to_string(),
            is_rtl: false,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
        LanguageConfig {
            code: "zh".to_string(),
            name: "Chinese (中文)".to_string(),
            is_rtl: false,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
        LanguageConfig {
            code: "ja".to_string(),
            name: "Japanese (日本語)".to_string(),
            is_rtl: false,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
        LanguageConfig {
            code: "ar".to_string(),
            name: "Arabic (العربية)".to_string(),
            is_rtl: true,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
        LanguageConfig {
            code: "he".to_string(),
            name: "Hebrew (עברית)".to_string(),
            is_rtl: true,
            font_family: Some("Liberation Sans".to_string()),
            enabled: false,
        },
    ]
}