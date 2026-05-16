"""Use to run a complex running model which incorporates the effects of terrain and weather on the runner's performance.

The model assumes throughout the run there are 3 main phases:
    1. Acceleration phase
    2. Constant velocity phase
    3. Deceleration phase

In addition to the Kellner model, this simulation incorporates:
    - Slope of the terrain (theta) which affects the runner's velocity and energy expenditure
    - Air resistance which is proportional to the square of the velocity and a drag coefficient
    - Heat stress which reduces the effective aerobic power supply (sigma) based on the Wet Bulb Globe Temperature (WBGT)
"""

import json
from logging import Logger
from pathlib import Path

import stride_sim_rust
from google.cloud import storage
from stride_sim_rust import CourseProfile, RunnerParams, SimulationConfig, Weather


def _config_to_dict(cfg: SimulationConfig) -> dict[str, float | int | str]:
    return {
        "target_dist": cfg.target_dist,
        "num_sim": cfg.num_sim,
        "dt": cfg.dt,
        "max_steps": cfg.max_steps,
        "sample_rate": cfg.sample_rate,
        "result_path": cfg.result_path,
    }

def _weather_to_dict(weather: Weather) -> dict[str, float | None]:
    return {
        "temperature": weather.temperature,
        "humidity": weather.humidity,
        "solar_radiation": weather.solar_radiation,
        "wind_speed": weather.wind_speed,
        "wind_azimuth": weather.wind_azimuth,
    }

def _course_to_dict(course: CourseProfile) -> dict[str, list[float] | None]:
    return {
        "distance": course.distance,
        "grade": course.grade,
        "azimuth": course.azimuth,
    }


class MonteCarloSimulation:
    """Wrapper class for running Monte Carlo simulations from the rust library."""

    def __init__(self, logger: Logger, runner_params: RunnerParams, cfg: SimulationConfig, weather: Weather, course: CourseProfile) -> None:
        """Use to initialize the simulation with the given configuration, input parameters, and optional course and weather data."""
        self.logger = logger
        self.runner_params = runner_params
        self.cfg = cfg
        self.weather = weather
        self.course = course

    def run(self) -> None:
        """Use to run the simulation."""
        stride_sim_rust.run_simulation(self.cfg, self.weather, self.course, self.runner_params)

    def run_collect(self) -> list[list[float]]:
        """Use to run the simulation and collect results in memory instead of writing to parquet."""
        return stride_sim_rust.run_simulation_collect(self.cfg, self.weather, self.course, self.runner_params)

    def save_to_cloud_results(self, bucket_name: str, simulation_folder: str, job_id: str, ts: str) -> None:
        """Use to save the results metadata, and configuration of the simulation."""
        # create a unique job id and base path for storing results in the bucket
        base_path = f"{simulation_folder}/{job_id}"
        self.logger.info(f"Saving results to cloud storage at: {bucket_name}/{base_path}")

        client = storage.Client()
        bucket = client.bucket(bucket_name)

        # save simulation configuration
        config_data = {
            "simulation_config": _config_to_dict(self.cfg),
            "weather": _weather_to_dict(self.weather),
            "course": _course_to_dict(self.course),
        }
        config_blob = bucket.blob(f"{base_path}/config.json")
        config_blob.upload_from_string(json.dumps(config_data), content_type="application/json")
        self.logger.info("Simulation configuration saved to cloud storage.")

        # save metadata
        metadata = {
            "job_id": job_id,
            "created_at": ts,
            "bucket": bucket_name,
        }

        metadata_blob = bucket.blob(f"{base_path}/metadata.json")
        metadata_blob.upload_from_string(
            json.dumps(metadata, indent=2),
            content_type="application/json",
        )
        self.logger.info("Simulation metadata saved to cloud storage.")

        # move the results from the temporary local path to cloud storage
        local_result_path = Path("/tmp/stride_sim/simulation_results.parquet") # noqa S108
        if local_result_path.exists():
            blob = bucket.blob(f"{base_path}/simulation_results.parquet")
            blob.upload_from_filename(local_result_path, content_type="application/octet-stream")
            local_result_path.unlink()  # delete the local file after uploading
            self.logger.info("Simulation results uploaded to cloud storage.")
        else:
            self.logger.warning(f"Local result file not found at {local_result_path}. No results uploaded to cloud storage.")

        # move the runner parameters results from the temporary local path to cloud storage
        local_runner_params_path = Path("/tmp/stride_sim/runner_params.parquet") # noqa S108
        if local_runner_params_path.exists():
            blob = bucket.blob(f"{base_path}/runner_params.parquet")
            blob.upload_from_filename(local_runner_params_path, content_type="application/octet-stream")
            local_runner_params_path.unlink()  # delete the local file after uploading
            self.logger.info("Runner parameters results uploaded to cloud storage.")
        else:
            self.logger.warning(f"Local runner parameters file not found at {local_runner_params_path}."
                                "No runner parameters results uploaded to cloud.")

    def save_to_local_results(self, bucket_name: str, simulation_folder: str, job_id: str, ts: str) -> None:
        """Use to save the results metadata, and configuration of the simulation."""
        # create a unique job id and base path for storing results in the bucket
        base_path = f"{bucket_name}/{simulation_folder}/{job_id}"
        self.logger.info(f"Saving results to local storage at: {base_path}")

        # create output folder if it doesn't exist
        output_folder_path = Path(base_path)
        output_folder_path.mkdir(parents=True, exist_ok=True)

        # save simulation configuration
        config_data = {
            "simulation_config": _config_to_dict(self.cfg),
            "weather": _weather_to_dict(self.weather),
            "course": _course_to_dict(self.course),
        }
        config_file_path = output_folder_path / "config.json"
        config_file_path.write_text(json.dumps(config_data, indent=4))
        self.logger.info("Simulation configuration saved to local storage.")

        # save metadata
        metadata = {
            "job_id": job_id,
            "created_at": ts,
            "bucket": bucket_name,
        }
        metadata_file_path = output_folder_path / "metadata.json"
        metadata_file_path.write_text(json.dumps(metadata, indent=4))
        self.logger.info("Simulation metadata saved to local storage.")
