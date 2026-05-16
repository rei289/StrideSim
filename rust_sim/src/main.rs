use crate::monte_carlo_simulation::{
    CourseProfile, MonteCarloSimulation, PacingStrategy, RunnerParams, SimulationConfig,
    SimulationInput, Weather,
};

use uom::si::f64::*;
use uom::si::length::meter;
use uom::si::time::second;
use uom::si::velocity::meter_per_second;
use uom::si::acceleration::meter_per_second_squared;
use uom::si::available_energy::joule_per_kilogram;
use uom::si::specific_power::watt_per_kilogram;
use uom::si::area::square_meter;
use uom::si::mass::kilogram;
use uom::si::mass_density::kilogram_per_cubic_meter;
use uom::si::heat_transfer::watt_per_square_meter_kelvin;
use uom::si::thermodynamic_temperature::degree_celsius;
use uom::si::heat_flux_density::watt_per_square_meter;
use uom::si::frequency::hertz;

mod monte_carlo_simulation;
mod param_config;

fn main() {
    let config = SimulationConfig {
        target_dist: Length::new::<meter>(42_195.0),
        num_sim: 5000,
        dt: Time::new::<second>(0.5),
        max_steps: 15_000,
        sample_rate: Time::new::<second>(10.0),
        result_path: Some("results.parquet".to_string()),
    };

    let runners = (0..config.num_sim)
        .map(|_| RunnerParams {
            runner_id: 0, // placeholder, will be set in MonteCarloSimulation::new
            f_max: Acceleration::new::<meter_per_second_squared>(3.0),
            e_init: AvailableEnergy::new::<joule_per_kilogram>(50_000.0),
            tau: Time::new::<second>(200.0),
            sigma: SpecificPower::new::<watt_per_kilogram>(10.0),
            k: Frequency::new::<hertz>(0.0), // placeholder, will be set in MonteCarloSimulation::new based on pacing strategy
            gamma: Frequency::new::<hertz>(0.1),
            drag_coefficient: 1.0,
            frontal_area: Area::new::<square_meter>(0.5),
            mass: Mass::new::<kilogram>(70.0),
            rho: MassDensity::new::<kilogram_per_cubic_meter>(1.225),
            convection: HeatTransfer::new::<watt_per_square_meter_kelvin>(10.0),
            alpha: 0.5,
            psi: 0.01,
            const_v: Velocity::new::<meter_per_second>(3.5),
            const_f: Acceleration::new::<meter_per_second_squared>(0.0), // placeholder, will be set in MonteCarloSimulation::new based on pacing strategy
            pacing: PacingStrategy::Constant,
        })
        .collect();

    let weather = Weather {
        temperature: ThermodynamicTemperature::new::<degree_celsius>(20.0),
        humidity: 0.50,
        solar_radiation: HeatFluxDensity::new::<watt_per_square_meter>(800.0),
        wind_speed: Velocity::new::<meter_per_second>(5.0),
        wind_azimuth: 90.0,
    };

    let course = CourseProfile {
        distance: vec![Length::new::<meter>(0.0), Length::new::<meter>(10_000.0), Length::new::<meter>(20_000.0), Length::new::<meter>(30_000.0), Length::new::<meter>(42_195.0)],
        grade: vec![0.0; 5],
        azimuth: vec![0.0; 5],
    };

    let input = SimulationInput {
        config,
        weather,
        course,
        runners,
    };

    let mut simulation = match MonteCarloSimulation::new(input) {
        Ok(sim) => sim,
        Err(err) => {
            eprintln!("failed to initialize simulation: {:?}", err);
            return;
        }
    };

    let _ = simulation.simulate();
}
