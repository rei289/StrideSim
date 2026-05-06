"""Test script to deploy directly in GCP."""
import os
import random
import time

import pyarrow.parquet as pq
import stride_sim_rust
from dotenv import load_dotenv

from src.simulation.monte_carlo_simulation import (
    MonteCarloSimulation,
)
from src.utilis.helper import job_id, time_now
from src.utilis.logger import StrideSimLogger


def load_runners_from_parquet(parquet_path: str, desired_num: int) -> list[stride_sim_rust.RunnerParams]:
    """Load runner parameters from a Parquet file and convert to RunnerParams objects."""
    table = pq.read_table(parquet_path)
    n = table.num_rows

    # choose random indices without replacement if there are more rows than desired_num
    indices = random.sample(range(n), k=desired_num) if n > desired_num else list(range(n))

    rows = table.take(indices).to_pydict()
    return [stride_sim_rust.RunnerParams(
        runner_id=int(row["runner_id"]),
        f_max=float(row["f_max"]),
        e_init=float(row["e_init"]),
        tau=float(row["tau"]),
        sigma=float(row["sigma"]),
        gamma=float(row["gamma"]),
        drag_coefficient=float(row["drag_coefficient"]),
        frontal_area=float(row["frontal_area"]),
        mass=float(row["mass"]),
        rho=float(row["rho"]),
        convection=float(row["convection"]),
        alpha=float(row["alpha"]),
        psi=float(row["psi"]),
        const_v=float(row["const_v"]),
        pacing=str(row["pacing"]),
    ) for row in rows]

config = stride_sim_rust.SimulationConfig(
    target_dist=43_000,
    num_sim=10_000,
    dt=0.1,
    max_steps=200_000,
    sample_rate=2.0,  # sample every 2 seconds
    result_path="test.parquet",
)

weather = stride_sim_rust.Weather(
    temperature=20.0,
    humidity=0.50,
    solar_radiation=800.0,
)

course = stride_sim_rust.CourseProfile(
    distance=[0.0, 10_000.0, 20_000.0, 30_000.0, 42_195.0],
    grade=[0.0, 0.0, 0.0, 0.0, 0.0],
    headwind=[0.0, 0.0, 0.0, 0.0, 0.0],
)

runners = [
    stride_sim_rust.RunnerParams(
        runner_id=i,
        f_max=10.0,
        e_init=2200.0,
        tau=1.0,
        sigma=28.0,
        gamma=5e-5,
        drag_coefficient=1.0,
        frontal_area=0.48,
        mass=70.0,
        rho=1.225,
        convection=10.0,
        alpha=0.7,
        psi=0.005,
        const_v=4.0,
        pacing="constant",
    )
    for i in range(config.num_sim)
]

if __name__ == "__main__":
    # get the time now
    ts = time_now()
    jid = job_id(ts)
    # define folder name for results
    folder_name = "03_simulations"

    # determine execution environment
    execution_env = os.getenv("EXECUTION_ENV", "local")

    if execution_env == "local":
        # save results to local file system
        load_dotenv()
        bucket_name = os.getenv("BUCKET_NAME", "local_results")

        logger_mgr = StrideSimLogger(execution_env=execution_env, bucket_name=bucket_name, folder_name=f"{folder_name}/{jid}")
        logger = logger_mgr.setup_logger()
        logger.info("Running in local environment")

        # change the simulation result path to the local bucket folder
        config.result_path = f"{bucket_name}/{folder_name}/{jid}"
    elif execution_env == "gcp":
        # get bucket name from environment variable
        bucket_name = os.getenv("BUCKET_NAME")

        logger_mgr = StrideSimLogger(execution_env=execution_env, bucket_name=bucket_name, folder_name=f"{folder_name}/{jid}")
        logger = logger_mgr.setup_logger()
        if not bucket_name:
            error = "The BUCKET_NAME environment variable is not set!"
            logger.error(error)
            raise ValueError(error)

        logger.info("Running in GCP environment")

        # change the simulation result path to a temporary local path
        config.result_path = "/tmp/stride_sim" # noqa S108
    else:
        logger_mgr = StrideSimLogger(execution_env=execution_env, bucket_name=None, folder_name=f"{folder_name}/{jid}")
        logger = logger_mgr.setup_logger()
        warn_message = f"Running in unknown environment: {execution_env}. Results will not be saved."
        logger.warning(warn_message)

    try:
        start_time = time.perf_counter()
        logger.info("Starting Monte Carlo Simulation...")

        sim = MonteCarloSimulation(logger, runners, config, weather, course)
        sim.run()

        end_time = time.perf_counter()
        elapsed_time = end_time - start_time
        logger.info(f"Monte Carlo Simulation completed in {elapsed_time:.2f} seconds.")
        logger.info("Monte Carlo Simulation completed successfully.")

        # save the results in their respective environment
        if execution_env == "local":
            # save results to local file system
            logger.info(f"Saving results to local bucket folder: {bucket_name}")
            sim.save_to_local_results(bucket_name, folder_name, jid, ts)
            logger.info("Results saved")
        elif execution_env == "gcp":
            # save results to cloud storage
            logger.info(f"Saving results to GCP bucket: {bucket_name}")
            sim.save_to_cloud_results(bucket_name, folder_name, jid, ts)
            logger.info("Results saved")
            # save logs to cloud storage
            logger.info("Uploading logs to GCP...")
            log_blob_path = logger_mgr.upload_log_to_gcs(bucket_name)
            logger.info(f"Logs uploaded to GCP at: {log_blob_path}")
    except Exception as e:
        error = f"An error occurred during the simulation: {e}"
        logger.exception(error)
        logger.exception("Simulation run failed")
    finally:
        logger.info("Closing logger")
        logger_mgr.close_logger(logger)


