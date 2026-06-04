# StrideSim GCP Terraform

This Terraform stack provisions the core GCP infrastructure used by StrideSim:

- API enablement for required Google services
- One GCS bucket for run/training/simulation outputs
- One Artifact Registry Docker repository
- Service account for GitHub Actions deployments
- Service account for Cloud Run job runtime
- GitHub OIDC Workload Identity Pool + Provider
- Secret Manager secret placeholders

## Prerequisites

- Terraform `>= 1.6`
- `gcloud` authenticated to the target project
- Permission to create IAM, storage, artifact registry, secret manager, and workload identity resources

## 1) Configure variables

Copy the example file and fill in real values:

```bash
cd infra/terraform
cp terraform.tfvars.example terraform.tfvars
```

Minimum values you must set in `terraform.tfvars`:

- `project_id`
- `bucket_name` (must be globally unique)
- `github_repository` (format: `owner/repo`)

## 2) Apply infrastructure

```bash
terraform init
terraform plan
terraform apply
```

## 3) Wire GitHub Actions settings

Your existing workflows expect these values:

- GitHub Secret `GCP_PROJECT_ID`: set to your project id
- GitHub Secret `GCP_PROJECT_NUMBER`: use Terraform output `project_number`
- GitHub Secret `GCP_SA_EMAIL`: use Terraform output `github_deployer_service_account_email`
- GitHub Secret `GCP_REPOSITORY`: use Terraform output `artifact_repository`
- GitHub Variable `GCP_BUCKET_NAME`: use Terraform output `bucket_name`

The workflows already reference:

- Workload identity pool id: `github-pool`
- Provider id: `github-provider`

Those are the defaults in this Terraform stack.

## 4) Add secret values in GCP Secret Manager

Terraform creates empty secret containers for:

- `STRAVA_CLIENT_ID`
- `STRAVA_CLIENT_SECRET`
- `STRAVA_REFRESH_TOKEN`
- `VISUAL_CROSSING_API_KEY`

Populate them after apply:

```bash
echo -n "<value>" | gcloud secrets versions add STRAVA_CLIENT_ID --data-file=- --project "<project-id>"
```

Repeat for each secret.

## Notes

- Bucket versioning is enabled by default.
- `bucket_force_destroy` defaults to `false` to avoid accidental data loss.
- Runtime service account has project-level access to Storage object operations and Secret Manager read access.
