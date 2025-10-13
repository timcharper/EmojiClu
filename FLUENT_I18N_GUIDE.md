# EmojiClu Internationalization (i18n) Guide

This document explains how internationalization (i18n) is implemented in EmojiClu using the [fluent-i18n](https://crates.io/crates/fluent-i18n) crate.

## Overview

EmojiClu now supports multiple languages using Mozilla's Fluent localization system. The setup includes:

- **English (en)** - Default/fallback language
- **Spanish (es)** - Complete translation
- **French (fr)** - Complete translation

## Directory Structure

```
locales/
├── en/
│   └── main.ftl
├── es/
│   └── main.ftl
└── fr/
    └── main.ftl
```

## How It Works

### 1. Initialization
The i18n system is initialized in `src/lib.rs`:
```rust
fluent_i18n::i18n!("locales", fallback = "en");
```

### 2. Usage in Code
Replace hardcoded strings with the `t!` macro:
```rust
use fluent_i18n::t;

// Before
Button::with_label("Submit")

// After  
Button::with_label(&t!("submit"))
```

### 3. Translation Keys
All translation keys are defined in `.ftl` files. For example, in `locales/en/main.ftl`:
```fluent
submit = Submit
menu-new-game = New Game
app-title = EmojiClu
```
