output "greeting" {
  description = "Rendered greeting text."
  value       = "Hello, ${local.normalized_greeting_name}."
}
