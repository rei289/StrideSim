output "project_number" {
  description = "Numeric GCP project number"
  value       = data.google_project.current.number
}

output "bucket_name" {
  description = "GCS bucket used by StrideSim"
  value       = google_storage_bucket.results.name
}

output "artifact_repository" {
  description = "Artifact Registry repository ID"
  value       = google_artifact_registry_repository.docker.repository_id
}

output "github_deployer_service_account_email" {
  description = "Service account email to store as GCP_SA_EMAIL in GitHub secrets"
  value       = google_service_account.deployer.email
}

output "runtime_service_account_email" {
  description = "Cloud Run runtime service account email"
  value       = google_service_account.runtime.email
}

output "workload_identity_provider_name" {
  description = "Provider resource name to use as workload_identity_provider in GitHub auth"
  value       = google_iam_workload_identity_pool_provider.github_provider.name
}
