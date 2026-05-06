# StrideSim

## Overview

StrideSim is a comprehensive running performance simulator that leverages historical running data to predict athlete performance across various conditions. By analyzing past running patterns, the program can simulate how a runner would perform under different scenarios. These include varied pacing strategies, terrain types, weather conditions, and variability in pace.

Whether you're a runner looking to optimize training, a coach planning race strategies, or a researcher studying performance patterns, StrideSim provides data-driven insights to help make informed decisions.

### Key Features

- **Performance Simulation**: Predict runner performance under different environmental and strategic conditions
- **Data-Driven Analysis**: Utilize historical running data to build accurate performance models
- **Multi-Condition Modeling**: Account for pacing strategies, terrain variations, weather impacts, and more

---

## Quick Start
To run this project, you can use the provided Docker containers for a streamlined setup. You can run the simulations, training, and data processing services locally or deploy them on Google Cloud Platform for scalable execution.


### Available Containers

- `Runs.Dockerfile`: Container for processing run data
- `Trainings.Dockerfile`: Container for training models
- `Simulations.Dockerfile`: Container for running simulations

### Local Execution
Before running locally, ensure you have Docker and Docker Compose installed. Then, you must set up your environment variables for secrets management.</p>
**Secrets Management (Local):** <br>
Create a `.env` file in the project root with your local configuration:
```
# .env
API_KEY=your_api_key_here
DATABASE_URL=your_database_url
# Add other required environment variables
```
The `.env` file is automatically loaded by Docker Compose and should **not** be committed to version control.

**Running Services Locally:** <br>
Run services locally using Docker Compose, where service_name corresponds to the desired container (`runs`, `trainings`, `simulations`):
```bash
docker-compose up <service_name> --build
```

### Cloud Execution (Google Cloud Platform)

Run StrideSim on GCP for scalable, serverless execution.

**Seccrets Management (GCP):** <br>
Use GCP Secret Manager and Github Actions to securely store and manage sensitive credentials. You can access these secrets in your application using the GCP SDK or environment variables configured in your deployment settings.

**Prerequisites:**
- Google Cloud account
- `gcloud` CLI installed and configured

**Setup:**

1. **Authenticate with GCP**
   ```bash
   gcloud auth login
   gcloud config set project YOUR_PROJECT_ID
   ```

2. **Link your GCP account**
   ```bash
   gcloud auth application-default login
   ```

3. **Configure Secrets**
   
   Store sensitive credentials in GitHub Secrets and GCP Secret Manager:
   - **GitHub Secrets**: Add secrets via repository Settings → Secrets and variables → Actions
   - **GCP Secret Manager**: Use `gcloud secrets create` to store credentials
   ```bash
   gcloud secrets create API_KEY --data-file=-
   gcloud secrets create DATABASE_URL --data-file=-
   ```

4. **Deploy and run on GCP**
    Once everything is set up, you can deploy your application using GitHub Actions. The GitHub Actions workflow will handle the deployment process, including building Docker images and deploying to GCP.

Refer to GCP documentation and the `docker-compose.yml` configuration for more details on cloud deployment options.

---
## Development Setup

### Prerequisites

- Python 3.8 or higher
- pip package manager

### Installation

1. **Clone the repository** (if applicable)
   ```bash
   git clone <repository-url>
   cd StrideSim
   ```

2. **Create a virtual environment**
   ```bash
   python -m venv venv
   ```

3. **Activate the virtual environment**
   - **Linux/macOS:**
     ```bash
     source venv/bin/activate
     ```
   - **Windows:**
     ```bash
     venv\Scripts\activate
     ```

4. **Install dependencies**
   ```bash
   pip install -r requirements-dev.txt
   pip install -e .
   ```

---

### Code Quality

This project uses **ruff** for linting and code quality checks.

Check for issues:
```bash
ruff check
```

Automatically fix issues:
```bash
ruff check --fix .
```

### Running Tests

```bash
pytest tests/
```

## Configuration

Project configuration is managed through YAML files in the `config/` directory:

- **constants.yml**: Fixed project constants
- **parameters.yml**: Adjustable simulation parameters
- **units.yml**: Unit system definitions

For detailed information about configuration options, see the config files or the documentation folder (coming soon).

---
## Project Structure

```
StrideSim/
├── src/                              # Main source code
│   ├── main_runs.py                  # Run execution
│   ├── main_simulations.py           # Simulation execution
│   ├── main_trainings.py             # Training execution
│   ├── model_training/               # ML model training
│   ├── process_runs/                 # Data processing for runs
│   ├── simulation/                   # Core simulation logic
│   └── utilis/                       # Utility functions
├── rust_sim/                         # Rust-based simulation engine
├── config/                           # Configuration files
│   ├── constants.yml                 # Project constants
│   ├── parameters.yml                # Simulation parameters
│   └── units.yml                     # Unit definitions
├── notebooks/                        # Jupyter notebooks for exploration
├── tests/                            # Test suite
├── requirements*.txt                 # Dependency specifications
└── docker-compose.yml                # Docker configuration
```
---

## Documentation

Detailed documentation including mathematical models, algorithms, and advanced concepts is available in the `documentation/` folder (to be added).

For quick reference:
- **Getting Started**: See this README
- **API Reference**: See inline code documentation
- **Mathematical Details**: See `documentation/` (coming soon)

---

## Dependencies

StrideSim uses separate requirement files for different purposes:

- `requirements-dev.txt`: Development dependencies (linting, testing)
- `requirements-sim.txt`: Simulation dependencies
- `requirements-train.txt`: Machine learning training dependencies
- `requirements-runs.txt`: Run data processing dependencies

---

## Contributing

Contributions are welcome! Please ensure:

1. Code passes linting checks (`ruff check --fix .`)
2. Tests pass (`pytest`)
3. Code follows project conventions

---

## License

[Add your license information here]

---

## Contact

[Add contact information or links here]
