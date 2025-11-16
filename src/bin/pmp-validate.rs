use pmp_gateway_api::config::Config;
use std::env;
use std::process;

fn main() {
    // Get config file path from args
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: pmp-validate <config-file>");
        eprintln!("\nExample:");
        eprintln!("  pmp-validate config.yaml");
        process::exit(1);
    }

    let config_path = &args[1];

    println!("Validating configuration file: {}", config_path);
    println!("{}", "=".repeat(60));

    // Try to load and validate the configuration
    match Config::from_yaml_file(config_path) {
        Ok(config) => {
            println!("✓ Configuration is valid!\n");

            // Print summary
            println!("Summary:");
            println!("  - Clients: {}", config.clients.len());
            println!("  - Routes: {}", config.routes.len());

            // Breakdown by client type
            let mut http_count = 0;
            let mut postgres_count = 0;
            let mut mysql_count = 0;
            let mut sqlite_count = 0;
            let mut mongodb_count = 0;
            let mut redis_count = 0;

            for client in config.clients.values() {
                match client {
                    pmp_gateway_api::config::ClientConfig::Http(_) => http_count += 1,
                    pmp_gateway_api::config::ClientConfig::Postgres(_) => postgres_count += 1,
                    pmp_gateway_api::config::ClientConfig::Mysql(_) => mysql_count += 1,
                    pmp_gateway_api::config::ClientConfig::Sqlite(_) => sqlite_count += 1,
                    pmp_gateway_api::config::ClientConfig::Mongodb(_) => mongodb_count += 1,
                    pmp_gateway_api::config::ClientConfig::Redis(_) => redis_count += 1,
                }
            }

            println!("\nClient breakdown:");
            if http_count > 0 {
                println!("  - HTTP: {}", http_count);
            }
            if postgres_count > 0 {
                println!("  - PostgreSQL: {}", postgres_count);
            }
            if mysql_count > 0 {
                println!("  - MySQL: {}", mysql_count);
            }
            if sqlite_count > 0 {
                println!("  - SQLite: {}", sqlite_count);
            }
            if mongodb_count > 0 {
                println!("  - MongoDB: {}", mongodb_count);
            }
            if redis_count > 0 {
                println!("  - Redis: {}", redis_count);
            }

            // Check for common issues
            let mut warnings = Vec::new();

            // Check for unused clients
            let used_clients: std::collections::HashSet<_> = config
                .routes
                .iter()
                .flat_map(|r| {
                    r.subrequests
                        .iter()
                        .map(|sr| sr.client_id.clone())
                        .collect::<Vec<_>>()
                })
                .collect();

            for client_id in config.clients.keys() {
                if !used_clients.contains(client_id) {
                    warnings.push(format!(
                        "Client '{}' is defined but not used in any route",
                        client_id
                    ));
                }
            }

            // Check for missing clients
            for route in &config.routes {
                for subrequest in &route.subrequests {
                    if !config.clients.contains_key(&subrequest.client_id) {
                        warnings.push(format!(
                            "Route '{}' references undefined client '{}'",
                            route.path, subrequest.client_id
                        ));
                    }
                }
            }

            if !warnings.is_empty() {
                println!("\n⚠ Warnings:");
                for warning in warnings {
                    println!("  - {}", warning);
                }
            }

            println!("\n{}", "=".repeat(60));
            println!("Configuration validation complete!");
            process::exit(0);
        }
        Err(e) => {
            eprintln!("✗ Configuration is invalid!\n");
            eprintln!("Error: {}", e);

            // Try to provide helpful error messages
            if e.to_string().contains("YAML") || e.to_string().contains("parsing") {
                eprintln!("\nHint: Check for YAML syntax errors:");
                eprintln!("  - Proper indentation (use spaces, not tabs)");
                eprintln!("  - Missing colons or dashes");
                eprintln!("  - Unclosed quotes");
            } else if e.to_string().contains("missing field") {
                eprintln!("\nHint: Required fields are missing.");
                eprintln!("  - Check the documentation for required fields");
            }

            eprintln!("\n{}", "=".repeat(60));
            process::exit(1);
        }
    }
}
