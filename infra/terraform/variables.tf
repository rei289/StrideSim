variable "project_id" {
  description = "GCP project ID where StrideSim infrastructure will be created"
  type        = string
}

variable "region" {
  description = "Primary region for Artifact Registry and Cloud Run jobs"
  type        = string
  default     = "us-central1"
}

variable "bucket_name" {
  description = "Name of the GCS bucket used by StrideSim for run, training, and simulation outputs"
  type        = string
}

variable "bucket_location" {
  description = "Location for the GCS bucket (for example US or us-central1)"
  type        = string
  default     = "US"
}

variable "bucket_force_destroy" {
  description = "Allow Terraform to destroy bucket even when it still has objects"
  type        = bool
  default     = false
}

variable "artifact_repository" {
  description = "Artifact Registry repository ID used by GitHub Actions"
  type        = string
  default     = "stridesim"
}

variable "github_repository" {
  description = "GitHub repository in owner/repo format allowed to federate into GCP"
  type        = string
}

variable "github_branch" {
  description = "Git branch allowed to use workload identity federation"
  type        = string
  default     = "main"
}

variable "workload_identity_pool_id" {
  description = "Workload Identity Pool ID"
  type        = string
  default     = "github-pool"
}

variable "workload_identity_provider_id" {
  description = "Workload Identity Provider ID"
  type        = string
  default     = "github-provider"
}

variable "deployer_service_account_id" {
  description = "Service account ID used by GitHub Actions for deployment"
  type        = string
  default     = "stridesim-gha"
}

variable "runtime_service_account_id" {
  description = "Service account ID used by Cloud Run jobs at runtime"
  type        = string
  default     = "stridesim-jobs"
}

variable "secret_names" {
  description = "Secret Manager secret names required by StrideSim"
  type        = list(string)
  default = [
    "STRAVA_CLIENT_ID",
    "STRAVA_CLIENT_SECRET",
    "STRAVA_REFRESH_TOKEN",
    "VISUAL_CROSSING_API_KEY",
  ]
}
