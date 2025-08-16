//! Authentication module for JWT token management

use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,    // User ID
    pub username: String,
    pub role: String,
    pub exp: usize,     // Expiration time
    pub iat: usize,     // Issued at
}

/// User registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// User login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub username: String,
    pub expires_at: String,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl User {
    /// Create a new user with hashed password
    pub fn new(username: String, email: String, password: String) -> Result<Self> {
        let password_hash = hash_password(&password)?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            username,
            email,
            password_hash,
            role: "user".to_string(),
            created_at: Utc::now(),
        })
    }

    /// Verify password against stored hash
    pub fn verify_password(&self, password: &str) -> bool {
        verify_password(password, &self.password_hash)
    }
}

/// Generate JWT token for a user
pub fn generate_token(user: &User, secret: &str) -> Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims {
        sub: user.id.clone(),
        username: user.username.clone(),
        role: user.role.clone(),
        exp: expiration,
        iat: Utc::now().timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )?;

    Ok(token)
}

/// Verify and decode JWT token
pub fn verify_token(token: &str, secret: &str) -> Result<TokenData<Claims>> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )?;

    Ok(token_data)
}

/// Hash password using a simple hash for demo purposes
/// In production, use bcrypt or argon2
fn hash_password(password: &str) -> Result<String> {
    // This is a simplified hash for demo purposes
    // In production, use bcrypt or argon2
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    let hash = hasher.finish();
    Ok(format!("simple_hash_{}", hash))
}

/// Verify password against hash
fn verify_password(password: &str, hash: &str) -> bool {
    if let Ok(computed_hash) = hash_password(password) {
        computed_hash == hash
    } else {
        false
    }
}

/// In-memory user store for demo purposes
/// In production, use a proper database
use std::sync::RwLock;
use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref USERS: RwLock<HashMap<String, User>> = RwLock::new(HashMap::new());
}

/// Store a user in memory
pub fn store_user(user: User) -> Result<()> {
    let mut users = USERS.write().unwrap();
    users.insert(user.username.clone(), user);
    Ok(())
}

/// Find user by username
pub fn find_user_by_username(username: &str) -> Option<User> {
    let users = USERS.read().unwrap();
    users.get(username).cloned()
}

/// Check if username exists
pub fn username_exists(username: &str) -> bool {
    let users = USERS.read().unwrap();
    users.contains_key(username)
}

/// Initialize with a default admin user
pub fn initialize_default_users() -> Result<()> {
    let admin_user = User {
        id: Uuid::new_v4().to_string(),
        username: "admin".to_string(),
        email: "admin@tradocflow.com".to_string(),
        password_hash: hash_password("admin123")?,
        role: "admin".to_string(),
        created_at: Utc::now(),
    };
    
    store_user(admin_user)?;
    Ok(())
}