use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use anyhow::{Result, Context};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::models::translation_models::{
    TranslationProject, TeamMember, UserRole, ProjectSettings, ValidationError
};
// Database migrations will be implemented later

/// Enhanced project service for translation management system
#[derive(Clone)]
pub struct ProjectService {
    projects_root: PathBuf,
}

/// Project creation request with all wizard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub priority: String,
    pub due_date: Option<String>,
    pub folder_path: PathBuf,
    pub template_id: String,
    pub translation_memory_option: String,
    pub source_language: String,
    pub target_languages: Vec<String>,
    pub team_members: Vec<TeamMemberRequest>,
}

/// Team member creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberRequest {
    pub name: String,
    pub email: String,
    pub role: String,
}

/// Project initialization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInitializationResult {
    pub project: TranslationProject,
    pub structure_created: bool,
    pub database_initialized: bool,
    pub chapters_created: Vec<String>,
}

impl ProjectService {
    /// Create a new project service
    pub fn new(projects_root: impl AsRef<Path>) -> Self {
        Self {
            projects_root: projects_root.as_ref().to_path_buf(),
        }
    }

    /// Create a new translation project with enhanced configuration
    pub async fn create_project(&self, request: CreateProjectRequest) -> Result<ProjectInitializationResult> {
        // Validate the request
        self.validate_create_request(&request)?;

        // Create the translation project model
        let mut project = TranslationProject::new(
            request.name.clone(),
            request.description.clone(),
            request.source_language.clone(),
            request.target_languages.clone(),
            request.folder_path.clone(),
        ).map_err(|e| anyhow::anyhow!("Failed to create project: {}", e))?;

        // Add team members
        for member_request in &request.team_members {
            let role = self.parse_user_role(&member_request.role)?;
            let languages = vec![request.source_language.clone()]; // Default to source language
            
            let team_member = TeamMember::new(
                Uuid::new_v4().to_string(),
                member_request.name.clone(),
                member_request.email.clone(),
                role,
                languages,
            ).map_err(|e| anyhow::anyhow!("Failed to create team member: {}", e))?;

            project.add_team_member(team_member)
                .map_err(|e| anyhow::anyhow!("Failed to add team member: {}", e))?;
        }

        // Create project directory structure
        let structure_created = self.create_project_structure(&project, &request).await?;

        // Initialize database
        let database_initialized = self.initialize_project_database(&project).await?;

        // Create initial chapters based on template
        let chapters_created = self.create_initial_chapters(&project, &request.template_id).await?;

        // Save project configuration
        self.save_project_configuration(&project, &request).await?;

        Ok(ProjectInitializationResult {
            project,
            structure_created,
            database_initialized,
            chapters_created,
        })
    }

    /// Validate the project creation request
    fn validate_create_request(&self, request: &CreateProjectRequest) -> Result<()> {
        // Validate project name
        if request.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Project name cannot be empty"));
        }

        // Validate folder path
        if !request.folder_path.is_absolute() {
            return Err(anyhow::anyhow!("Project folder path must be absolute"));
        }

        // Check if folder already exists and is not empty
        if request.folder_path.exists() {
            if let Ok(entries) = fs::read_dir(&request.folder_path) {
                if entries.count() > 0 {
                    return Err(anyhow::anyhow!("Project folder must be empty"));
                }
            }
        }

        // Validate languages
        if request.source_language.trim().is_empty() {
            return Err(anyhow::anyhow!("Source language must be specified"));
        }

        if request.target_languages.is_empty() {
            return Err(anyhow::anyhow!("At least one target language must be specified"));
        }

        if request.target_languages.contains(&request.source_language) {
            return Err(anyhow::anyhow!("Source language cannot be a target language"));
        }

        // Validate team members
        for member in &request.team_members {
            if member.name.trim().is_empty() {
                return Err(anyhow::anyhow!("Team member name cannot be empty"));
            }

            if !member.email.contains('@') {
                return Err(anyhow::anyhow!("Invalid email format for team member: {}", member.name));
            }

            if !["Translator", "Reviewer", "Project Manager", "Admin"].contains(&member.role.as_str()) {
                return Err(anyhow::anyhow!("Invalid role for team member: {}", member.name));
            }
        }

        Ok(())
    }

    /// Create the project directory structure
    async fn create_project_structure(&self, project: &TranslationProject, request: &CreateProjectRequest) -> Result<bool> {
        let project_path = &request.folder_path;

        // Create main project directory
        fs::create_dir_all(project_path)
            .context("Failed to create project directory")?;

        // Create subdirectories
        let subdirs = [
            "chapters",
            "translation_memory",
            "terminology", 
            "exports",
            "collaboration",
            "settings",
        ];

        for subdir in &subdirs {
            fs::create_dir_all(project_path.join(subdir))
                .context(format!("Failed to create {} directory", subdir))?;
        }

        // Create language-specific chapter directories
        let all_languages = std::iter::once(request.source_language.clone())
            .chain(request.target_languages.iter().cloned())
            .collect::<Vec<_>>();

        let chapters_path = project_path.join("chapters");
        for language in &all_languages {
            fs::create_dir_all(chapters_path.join(language))
                .context(format!("Failed to create chapter directory for language: {}", language))?;
        }

        // Create export subdirectories
        let export_path = project_path.join("exports");
        for format in &["pdf", "html", "docx"] {
            fs::create_dir_all(export_path.join(format))
                .context(format!("Failed to create export directory for format: {}", format))?;
        }

        Ok(true)
    }

    /// Initialize project database with translation memory and terminology tables
    async fn initialize_project_database(&self, project: &TranslationProject) -> Result<bool> {
        let _db_path = project.project_path.join("translation_memory").join("project.db");
        
        // TODO: Run database migrations to create tables
        // This will be implemented when the database migration system is ready
        
        // Create initial Parquet files for translation memory
        let parquet_path = project.project_path.join("translation_memory");
        
        // Create empty translation units parquet file
        let units_path = parquet_path.join("units.parquet");
        self.create_empty_parquet_file(&units_path, "translation_units").await?;

        // Create empty chunks parquet file
        let chunks_path = parquet_path.join("chunks.parquet");
        self.create_empty_parquet_file(&chunks_path, "chunks").await?;

        // Create empty terminology parquet file
        let terminology_path = project.project_path.join("terminology").join("terms.parquet");
        self.create_empty_parquet_file(&terminology_path, "terminology").await?;

        Ok(true)
    }

    /// Create initial chapters based on the selected template
    async fn create_initial_chapters(&self, project: &TranslationProject, template_id: &str) -> Result<Vec<String>> {
        let chapters = self.get_template_chapters(template_id);
        let mut created_chapters = Vec::new();

        let chapters_path = project.project_path.join("chapters");
        let all_languages = std::iter::once(project.source_language.clone())
            .chain(project.target_languages.iter().cloned())
            .collect::<Vec<_>>();

        for (index, chapter_info) in chapters.iter().enumerate() {
            let chapter_number = index + 1;
            let filename = format!("{:02}_{}.md", chapter_number, chapter_info.slug);

            for language in &all_languages {
                let lang_path = chapters_path.join(language);
                let file_path = lang_path.join(&filename);

                let content = if language == &project.source_language {
                    // Create content for source language
                    format!("# {}\n\n{}\n\n*This chapter is ready for content creation.*", 
                           chapter_info.title, chapter_info.description)
                } else {
                    // Create placeholder for target languages
                    format!("# {}\n\n*Translation needed from {}*", 
                           chapter_info.title, project.source_language)
                };

                fs::write(&file_path, content)
                    .context(format!("Failed to create chapter file: {}", filename))?;
            }

            created_chapters.push(filename);
        }

        Ok(created_chapters)
    }

    /// Save project configuration to JSON file
    async fn save_project_configuration(&self, project: &TranslationProject, request: &CreateProjectRequest) -> Result<()> {
        let config_path = project.project_path.join("project.json");
        
        let config = ProjectConfiguration {
            project: project.clone(),
            template_id: request.template_id.clone(),
            translation_memory_option: request.translation_memory_option.clone(),
            created_at: Utc::now(),
            version: "1.0".to_string(),
        };

        let config_json = serde_json::to_string_pretty(&config)
            .context("Failed to serialize project configuration")?;

        fs::write(&config_path, config_json)
            .context("Failed to write project configuration")?;

        // Save language configuration
        let lang_config_path = project.project_path.join("settings").join("languages.json");
        let lang_config = LanguageConfiguration {
            source_language: project.source_language.clone(),
            target_languages: project.target_languages.clone(),
            language_codes: self.get_language_codes(&project.source_language, &project.target_languages),
        };

        let lang_config_json = serde_json::to_string_pretty(&lang_config)
            .context("Failed to serialize language configuration")?;

        fs::write(&lang_config_path, lang_config_json)
            .context("Failed to write language configuration")?;

        Ok(())
    }

    /// Get template chapters based on template ID
    fn get_template_chapters(&self, template_id: &str) -> Vec<ChapterTemplate> {
        match template_id {
            "technical_manual" => vec![
                ChapterTemplate {
                    title: "Introduction".to_string(),
                    slug: "introduction".to_string(),
                    description: "Overview of the system and its capabilities".to_string(),
                },
                ChapterTemplate {
                    title: "System Requirements".to_string(),
                    slug: "system-requirements".to_string(),
                    description: "Hardware and software requirements".to_string(),
                },
                ChapterTemplate {
                    title: "Installation Guide".to_string(),
                    slug: "installation-guide".to_string(),
                    description: "Step-by-step installation instructions".to_string(),
                },
                ChapterTemplate {
                    title: "Configuration".to_string(),
                    slug: "configuration".to_string(),
                    description: "System configuration and setup".to_string(),
                },
                ChapterTemplate {
                    title: "API Reference".to_string(),
                    slug: "api-reference".to_string(),
                    description: "Complete API documentation".to_string(),
                },
                ChapterTemplate {
                    title: "Troubleshooting".to_string(),
                    slug: "troubleshooting".to_string(),
                    description: "Common issues and solutions".to_string(),
                },
            ],
            "user_guide" => vec![
                ChapterTemplate {
                    title: "Getting Started".to_string(),
                    slug: "getting-started".to_string(),
                    description: "Quick start guide for new users".to_string(),
                },
                ChapterTemplate {
                    title: "Basic Features".to_string(),
                    slug: "basic-features".to_string(),
                    description: "Essential features and functionality".to_string(),
                },
                ChapterTemplate {
                    title: "Advanced Features".to_string(),
                    slug: "advanced-features".to_string(),
                    description: "Advanced tools and capabilities".to_string(),
                },
                ChapterTemplate {
                    title: "Tips and Tricks".to_string(),
                    slug: "tips-and-tricks".to_string(),
                    description: "Best practices and helpful tips".to_string(),
                },
                ChapterTemplate {
                    title: "FAQ".to_string(),
                    slug: "faq".to_string(),
                    description: "Frequently asked questions".to_string(),
                },
            ],
            "training_material" => vec![
                ChapterTemplate {
                    title: "Course Overview".to_string(),
                    slug: "course-overview".to_string(),
                    description: "Introduction to the training course".to_string(),
                },
                ChapterTemplate {
                    title: "Learning Objectives".to_string(),
                    slug: "learning-objectives".to_string(),
                    description: "What you will learn in this course".to_string(),
                },
                ChapterTemplate {
                    title: "Module 1 - Basics".to_string(),
                    slug: "module-1-basics".to_string(),
                    description: "Fundamental concepts and principles".to_string(),
                },
                ChapterTemplate {
                    title: "Module 2 - Intermediate".to_string(),
                    slug: "module-2-intermediate".to_string(),
                    description: "Intermediate topics and applications".to_string(),
                },
                ChapterTemplate {
                    title: "Module 3 - Advanced".to_string(),
                    slug: "module-3-advanced".to_string(),
                    description: "Advanced techniques and best practices".to_string(),
                },
                ChapterTemplate {
                    title: "Assessment".to_string(),
                    slug: "assessment".to_string(),
                    description: "Knowledge check and evaluation".to_string(),
                },
            ],
            _ => vec![
                ChapterTemplate {
                    title: "Introduction".to_string(),
                    slug: "introduction".to_string(),
                    description: "Welcome to your new project".to_string(),
                },
            ],
        }
    }

    /// Parse user role string to UserRole enum
    fn parse_user_role(&self, role_str: &str) -> Result<UserRole> {
        match role_str {
            "Translator" => Ok(UserRole::Translator),
            "Reviewer" => Ok(UserRole::Reviewer),
            "Project Manager" => Ok(UserRole::ProjectManager),
            "Admin" => Ok(UserRole::Admin),
            _ => Err(anyhow::anyhow!("Invalid user role: {}", role_str)),
        }
    }

    /// Get language codes mapping
    fn get_language_codes(&self, source: &str, targets: &[String]) -> HashMap<String, String> {
        let mut codes = HashMap::new();
        
        // Map language names to ISO codes
        let language_map = [
            ("English", "en"),
            ("Spanish", "es"),
            ("French", "fr"),
            ("German", "de"),
            ("Italian", "it"),
            ("Portuguese", "pt"),
            ("Dutch", "nl"),
            ("Chinese", "zh"),
            ("Japanese", "ja"),
        ].iter().cloned().collect::<HashMap<_, _>>();

        if let Some(code) = language_map.get(source) {
            codes.insert(source.to_string(), code.to_string());
        }

        for target in targets {
            if let Some(code) = language_map.get(target.as_str()) {
                codes.insert(target.clone(), code.to_string());
            }
        }

        codes
    }

    /// Discover existing translation projects in a directory or system
    pub async fn discover_projects(&self, search_paths: Option<Vec<PathBuf>>) -> Result<Vec<ProjectSummary>> {
        let mut projects = Vec::new();
        
        // Default search paths if none provided
        let paths = search_paths.unwrap_or_else(|| vec![
            self.projects_root.clone(),
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("Documents"),
        ]);
        
        for search_path in paths {
            if !search_path.exists() {
                continue;
            }
            
            // Look for project.json files in directories
            if let Ok(entries) = fs::read_dir(&search_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let project_config = path.join("project.json");
                        if project_config.exists() {
                            if let Ok(summary) = self.load_project_summary(&project_config).await {
                                projects.push(summary);
                            }
                        }
                    }
                }
            }
        }
        
        // Sort projects by last modified time (most recent first)
        projects.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
        
        Ok(projects)
    }

    /// Load a project from its configuration file
    pub async fn load_project(&self, project_path: &Path) -> Result<TranslationProject> {
        let config_path = project_path.join("project.json");
        
        if !config_path.exists() {
            return Err(anyhow::anyhow!("Project configuration not found: {:?}", config_path));
        }
        
        let config_content = fs::read_to_string(&config_path)
            .context("Failed to read project configuration")?;
            
        let config: ProjectConfiguration = serde_json::from_str(&config_content)
            .context("Failed to parse project configuration")?;
            
        Ok(config.project)
    }

    /// Load a project summary from its configuration file
    async fn load_project_summary(&self, config_path: &Path) -> Result<ProjectSummary> {
        let config_content = fs::read_to_string(config_path)
            .context("Failed to read project configuration")?;
            
        let config: ProjectConfiguration = serde_json::from_str(&config_content)
            .context("Failed to parse project configuration")?;
        
        let project_dir = config_path.parent().unwrap();
        let last_modified = fs::metadata(config_path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            
        // Count chapters
        let chapters_path = project_dir.join("chapters");
        let chapter_count = if chapters_path.exists() {
            self.count_markdown_files(&chapters_path).unwrap_or(0)
        } else {
            0
        };
        
        Ok(ProjectSummary {
            name: config.project.name,
            description: config.project.description,
            path: project_dir.to_path_buf(),
            source_language: config.project.source_language,
            target_languages: config.project.target_languages,
            team_member_count: config.project.team_members.len(),
            chapter_count,
            template_id: config.template_id,
            last_modified,
            created_at: config.created_at,
        })
    }

    /// Count markdown files in a directory recursively
    fn count_markdown_files(&self, dir: &Path) -> Result<usize> {
        let mut count = 0;
        
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                    count += 1;
                } else if path.is_dir() {
                    count += self.count_markdown_files(&path)?;
                }
            }
        }
        
        Ok(count)
    }

    /// Create an empty Parquet file with the appropriate schema
    async fn create_empty_parquet_file(&self, path: &Path, schema_type: &str) -> Result<()> {
        // For now, create a placeholder file
        // In a real implementation, this would create a proper Parquet file with the correct schema
        fs::write(path, format!("# Empty {} Parquet file\n# Will be populated by the translation memory service", schema_type))
            .context(format!("Failed to create empty Parquet file: {:?}", path))?;
        
        Ok(())
    }
}

/// Project configuration stored in project.json
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectConfiguration {
    project: TranslationProject,
    template_id: String,
    translation_memory_option: String,
    created_at: chrono::DateTime<chrono::Utc>,
    version: String,
}

/// Language configuration stored in settings/languages.json
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LanguageConfiguration {
    source_language: String,
    target_languages: Vec<String>,
    language_codes: HashMap<String, String>,
}

/// Chapter template for project initialization
#[derive(Debug, Clone)]
struct ChapterTemplate {
    title: String,
    slug: String,
    description: String,
}

/// Project summary for project browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub description: Option<String>,
    pub path: PathBuf,
    pub source_language: String,
    pub target_languages: Vec<String>,
    pub team_member_count: usize,
    pub chapter_count: usize,
    pub template_id: String,
    pub last_modified: std::time::SystemTime,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_project_with_enhanced_wizard() {
        let temp_dir = TempDir::new().unwrap();
        let service = ProjectService::new(temp_dir.path());

        let project_path = temp_dir.path().join("test-project");
        
        let request = CreateProjectRequest {
            name: "Test Translation Project".to_string(),
            description: Some("A comprehensive test project".to_string()),
            priority: "high".to_string(),
            due_date: None,
            folder_path: project_path.clone(),
            template_id: "technical_manual".to_string(),
            translation_memory_option: "new".to_string(),
            source_language: "English".to_string(),
            target_languages: vec!["Spanish".to_string(), "French".to_string()],
            team_members: vec![
                TeamMemberRequest {
                    name: "John Translator".to_string(),
                    email: "john@example.com".to_string(),
                    role: "Translator".to_string(),
                },
                TeamMemberRequest {
                    name: "Jane Reviewer".to_string(),
                    email: "jane@example.com".to_string(),
                    role: "Reviewer".to_string(),
                },
            ],
        };

        let result = service.create_project(request).await.unwrap();

        // Verify project was created
        assert_eq!(result.project.name, "Test Translation Project");
        assert_eq!(result.project.source_language, "English");
        assert_eq!(result.project.target_languages, vec!["Spanish", "French"]);
        assert_eq!(result.project.team_members.len(), 2);
        assert!(result.structure_created);
        assert!(result.database_initialized);
        assert_eq!(result.chapters_created.len(), 6); // Technical manual has 6 chapters

        // Verify directory structure
        assert!(project_path.join("chapters").exists());
        assert!(project_path.join("chapters/English").exists());
        assert!(project_path.join("chapters/Spanish").exists());
        assert!(project_path.join("chapters/French").exists());
        assert!(project_path.join("translation_memory").exists());
        assert!(project_path.join("terminology").exists());
        assert!(project_path.join("exports").exists());
        assert!(project_path.join("project.json").exists());

        // Verify chapters were created
        assert!(project_path.join("chapters/English/01_introduction.md").exists());
        assert!(project_path.join("chapters/Spanish/01_introduction.md").exists());
        assert!(project_path.join("chapters/French/01_introduction.md").exists());

        println!("✅ Enhanced project creation test passed!");
    }

    #[tokio::test]
    async fn test_project_validation() {
        let temp_dir = TempDir::new().unwrap();
        let service = ProjectService::new(temp_dir.path());

        // Test empty name validation
        let invalid_request = CreateProjectRequest {
            name: "".to_string(),
            description: None,
            priority: "medium".to_string(),
            due_date: None,
            folder_path: temp_dir.path().join("test"),
            template_id: "blank".to_string(),
            translation_memory_option: "new".to_string(),
            source_language: "English".to_string(),
            target_languages: vec!["Spanish".to_string()],
            team_members: vec![],
        };

        let result = service.create_project(invalid_request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Project name cannot be empty"));

        println!("✅ Project validation test passed!");
    }
}