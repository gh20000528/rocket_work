
# Rocket User Registration API

## Features
- User Registration
- User Login
- User Information Retrieval
- User Password Management
- Role Management
- Permission Management
- Captcha Generation and Validation

## Technologies Used
- Rust
- Rocket
- SQLx
- bcrypt
- serde
- dotenv
- PostgreSQL (or your chosen database)
- Prerequisites
- Rust (latest stable version)
- Cargo (comes with Rust)
- PostgreSQL (or another database supported by SQLx)
- dotenv
## Getting Started
- run code -> cargo watch -q -c -w src/ -x run

```
src
├── controllers
│   ├── mod.rs
│   ├── permission_controller.rs
│   └── user_controller.rs
├── db.rs
├── main.rs
├── models
│   ├── captcha.rs
│   ├── mod.rs
│   ├── permission.rs
│   └── user.rs
├── responses
│   ├── mod.rs
│   └── response.rs
└── tools
    ├── jwt.rs
    ├── mod.rs
    └── permission_control.rs
```