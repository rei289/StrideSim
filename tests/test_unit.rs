use stride_sim_rust::monte_carlo_simulation::{
    MonteCarloSimulation, SimulationConfig, SimulationInput, Weather, CourseProfile, RunnerParams, PacingStrategy
};
use stride_sim_rust::param_config::get_min_max_values;
use tempfile::tempdir;

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

// entire simulation pipeline is tested here, from input struct to output file writing
#[test]
fn simulation_writes_output_without_error() {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let result_path = temp_dir.path().join("test-results.parquet");
    let config = SimulationConfig {
        target_dist: Length::new::<meter>(1000.0),
        num_sim: 2,
        dt: Time::new::<second>(1.0),
        max_steps: 2000,
        sample_rate: Time::new::<second>(5.0),
        result_path: Some(result_path.to_string_lossy().to_string()),
    };

    let runners = (0..config.num_sim).map(|i| RunnerParams {
        runner_id: i as u32,
        f_max: Acceleration::new::<meter_per_second_squared>(3.0),
        e_init: AvailableEnergy::new::<joule_per_kilogram>(50000.0),
        tau: Time::new::<second>(200.0),
        sigma: SpecificPower::new::<watt_per_kilogram>(10.0),
        k: Frequency::new::<hertz>(0.0),
        gamma: Frequency::new::<hertz>(0.1),
        drag_coefficient: 1.0,
        frontal_area: Area::new::<square_meter>(0.5),
        mass: Mass::new::<kilogram>(70.0),
        rho: MassDensity::new::<kilogram_per_cubic_meter>(1.225),
        convection: HeatTransfer::new::<watt_per_square_meter_kelvin>(10.0),
        alpha: 0.5,
        psi: 0.01,
        const_v: Velocity::new::<meter_per_second>(3.5),
        const_f: Acceleration::new::<meter_per_second_squared>(0.0),
        pacing: PacingStrategy::Constant,
    }).collect();

    let input = SimulationInput {
        config,
        weather: Weather {
            temperature: ThermodynamicTemperature::new::<degree_celsius>(20.0),
            humidity: 50.0,
            solar_radiation: HeatFluxDensity::new::<watt_per_square_meter>(700.0),
            wind_speed: Velocity::new::<meter_per_second>(5.0),
            wind_azimuth: 90.0,
        },
        course: CourseProfile {
            distance: vec![Length::new::<meter>(0.0), Length::new::<meter>(1000.0)],
            grade: vec![0.0, 0.0],
            azimuth: vec![0.0, 45.0],
        },
        runners,
    };

    let mut sim = MonteCarloSimulation::new(input).expect("init failed");
    sim.simulate().expect("simulate failed");
    assert!(result_path.exists());
}

// specific unit tests input variables
#[test]
fn runner_params_within_valid_range() {
    let runner =  RunnerParams {
        runner_id: 1 as u32,
        f_max: Acceleration::new::<meter_per_second_squared>(3.0),
        e_init: AvailableEnergy::new::<joule_per_kilogram>(5000.0),
        tau: Time::new::<second>(5.0),
        sigma: SpecificPower::new::<watt_per_kilogram>(5.0),
        k: Frequency::new::<hertz>(0.1),
        gamma: Frequency::new::<hertz>(0.1),
        drag_coefficient: 1.0,
        frontal_area: Area::new::<square_meter>(0.5),
        mass: Mass::new::<kilogram>(70.0),
        rho: MassDensity::new::<kilogram_per_cubic_meter>(1.225),
        convection: HeatTransfer::new::<watt_per_square_meter_kelvin>(10.0),
        alpha: 0.5,
        psi: 0.01,
        const_v: Velocity::new::<meter_per_second>(3.5),
        const_f: Acceleration::new::<meter_per_second_squared>(0.0),
        pacing: PacingStrategy::Constant,
    };

    // check that all parameters are within expected ranges
    // f_max
    let Ok((f_max_min, f_max_max)) = get_min_max_values("physical", "acceleration") else {
        panic!("failed to get min/max values for f_max");
    };
    assert!(runner.f_max.get::<meter_per_second_squared>() >= f_max_min && runner.f_max.get::<meter_per_second_squared>() <= f_max_max);
    // e_init
    let Ok((e_init_min, e_init_max)) = get_min_max_values("physical", "energy") else {
        panic!("failed to get min/max values for e_init");
    };
    assert!(runner.e_init.get::<joule_per_kilogram>() >= e_init_min && runner.e_init.get::<joule_per_kilogram>() <= e_init_max);
    // tau
    let Ok((tau_min, tau_max)) = get_min_max_values("physical", "tau") else {
        panic!("failed to get min/max values for tau");
    };
    assert!(runner.tau.get::<second>() >= tau_min && runner.tau.get::<second>() <= tau_max);
    // sigma
    let Ok((sigma_min, sigma_max)) = get_min_max_values("physical", "sigma") else {
        panic!("failed to get min/max values for sigma");
    };
    assert!(runner.sigma.get::<watt_per_kilogram>() >= sigma_min && runner.sigma.get::<watt_per_kilogram>() <= sigma_max);
    // k
    let Ok((k_min, k_max)) = get_min_max_values("physical", "gamma") else {
        panic!("failed to get min/max values for k");
    };
    assert!(runner.k.get::<hertz>() >= k_min/2.0 && runner.k.get::<hertz>() <= k_max*2.0);
    // gamma
    let Ok((gamma_min, gamma_max)) = get_min_max_values("physical", "gamma") else {
        panic!("failed to get min/max values for gamma");
    };
    assert!(runner.gamma.get::<hertz>() >= gamma_min && runner.gamma.get::<hertz>() <= gamma_max);
    // drag_coefficient
    let Ok((drag_coefficient_min, drag_coefficient_max)) = get_min_max_values("environmental", "drag_coefficient") else {
        panic!("failed to get min/max values for drag_coefficient");
    };
    assert!(runner.drag_coefficient >= drag_coefficient_min && runner.drag_coefficient <= drag_coefficient_max);
    // frontal_area
    let Ok((frontal_area_min, frontal_area_max)) = get_min_max_values("physical", "area") else {
        panic!("failed to get min/max values for frontal_area");
    };
    assert!(runner.frontal_area.get::<square_meter>() >= frontal_area_min && runner.frontal_area.get::<square_meter>() <= frontal_area_max);
    // mass
    let Ok((mass_min, mass_max)) = get_min_max_values("physical", "mass") else {
        panic!("failed to get min/max values for mass");
    };
    assert!(runner.mass.get::<kilogram>() >= mass_min && runner.mass.get::<kilogram>() <= mass_max);
    // rho
    let Ok((rho_min, rho_max)) = get_min_max_values("environmental", "air_density") else {
        panic!("failed to get min/max values for rho");
    };
    assert!(runner.rho.get::<kilogram_per_cubic_meter>() >= rho_min && runner.rho.get::<kilogram_per_cubic_meter>() <= rho_max);
    // convection
    let Ok((convection_min, convection_max)) = get_min_max_values("environmental", "convection") else {
        panic!("failed to get min/max values for convection");
    };
    assert!(runner.convection.get::<watt_per_square_meter_kelvin>() >= convection_min && runner.convection.get::<watt_per_square_meter_kelvin>() <= convection_max);
    // alpha
    let Ok((alpha_min, alpha_max)) = get_min_max_values("environmental", "alpha") else {
        panic!("failed to get min/max values for alpha");
    };
    assert!(runner.alpha >= alpha_min && runner.alpha <= alpha_max);
    // psi
    let Ok((psi_min, psi_max)) = get_min_max_values("environmental", "psi") else {
        panic!("failed to get min/max values for psi");
    };
    assert!(runner.psi >= psi_min && runner.psi <= psi_max);
    // const_v
    let Ok((const_v_min, const_v_max)) = get_min_max_values("physical", "velocity") else {
        panic!("failed to get min/max values for const_v");
    };
    assert!(runner.const_v.get::<meter_per_second>() >= const_v_min && runner.const_v.get::<meter_per_second>() <= const_v_max);
    
}

// test that invalid simulation inputs are properly rejected with errors
#[test]
fn invalid_simulation_inputs_are_rejected() {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let result_path = temp_dir.path().join("test-results.parquet");

    let mut input = SimulationInput {
        config: SimulationConfig {
            target_dist: Length::new::<meter>(1000.0),
            num_sim: 1,
            dt: Time::new::<second>(1.0),
            max_steps: 2000,
            sample_rate: Time::new::<second>(5.0),
            result_path: Some(result_path.to_string_lossy().to_string()),
        },
        weather: Weather {
            temperature: ThermodynamicTemperature::new::<degree_celsius>(20.0),
            humidity: 50.0,
            solar_radiation: HeatFluxDensity::new::<watt_per_square_meter>(700.0),
            wind_speed: Velocity::new::<meter_per_second>(5.0),
            wind_azimuth: 90.0,
        },
        course: CourseProfile {
            distance: vec![Length::new::<meter>(0.0), Length::new::<meter>(1000.0)],
            grade: vec![0.0, 0.0],
            azimuth: vec![0.0, 45.0, 90.0, 135.0],
        },
        runners: vec![RunnerParams {
            runner_id: 1 as u32,
            f_max: Acceleration::new::<meter_per_second_squared>(1.0),
            e_init: AvailableEnergy::new::<joule_per_kilogram>(5000.0),
            tau: Time::new::<second>(5.0),
            sigma: SpecificPower::new::<watt_per_kilogram>(5.0),
            k: Frequency::new::<hertz>(0.1),
            gamma: Frequency::new::<hertz>(0.1),
            drag_coefficient: 1.0,
            frontal_area: Area::new::<square_meter>(0.5),
            mass: Mass::new::<kilogram>(70.0),
            rho: MassDensity::new::<kilogram_per_cubic_meter>(1.225),
            convection: HeatTransfer::new::<watt_per_square_meter_kelvin>(10.0),
            alpha: 0.5,
            psi: 0.01,
            const_v: Velocity::new::<meter_per_second>(3.5),
            const_f: Acceleration::new::<meter_per_second_squared>(0.0),
            pacing: PacingStrategy::Constant,
        }],
    };

    // change the num_sim to 0
    input.config.num_sim = 0;
    let sim_result = MonteCarloSimulation::new(input.clone());
    assert!(sim_result.is_err());

    // change max_steps to 0
    input.config.num_sim = 1; // reset to valid value
    input.config.max_steps = 0;
    let sim_result = MonteCarloSimulation::new(input.clone());
    assert!(sim_result.is_err());

    // change dt to 0
    input.config.max_steps = 2000; // reset to valid value
    input.config.dt = Time::new::<second>(0.0);
    let sim_result = MonteCarloSimulation::new(input.clone());
    assert!(sim_result.is_err());

    // change sample_rate to 0
    input.config.dt = Time::new::<second>(1.0); // reset to valid value
    input.config.sample_rate = Time::new::<second>(0.0);
    let sim_result = MonteCarloSimulation::new(input.clone());
    assert!(sim_result.is_err());

    // change course distance and grade vectors to different lengths
    input.config.sample_rate = Time::new::<second>(5.0); // reset to valid value
    input.course.grade = vec![0.0]; // change to different length than distance
    let sim_result = MonteCarloSimulation::new(input.clone());
    assert!(sim_result.is_err());

    // change course distance and azimuth vectors to different lengths
    input.course.grade = vec![0.0, 0.0]; // reset to valid value
    input.course.azimuth = vec![0.0]; // change to different length than distance
    let sim_result = MonteCarloSimulation::new(input.clone());
    assert!(sim_result.is_err());

    // change weather humidity to invalid value
    input.course.azimuth = vec![0.0, 45.0, 90.0, 135.0]; // reset to valid value
    input.weather.humidity = -10.0; // invalid humidity
    let sim_result = MonteCarloSimulation::new(input.clone());
    assert!(sim_result.is_err());

    // change the solar radiation to a negative value
    input.weather.humidity = 50.0; // reset to valid value
    input.weather.solar_radiation = HeatFluxDensity::new::<watt_per_square_meter>(-100.0); // invalid solar radiation
    let sim_result = MonteCarloSimulation::new(input.clone());
    assert!(sim_result.is_err());
}

// test that the get_grade function properly finds the correct grade for a given distance
#[test]
fn get_grade_and_headwind_use_closest_distance_and_advance_indices() {
    let input = SimulationInput {
        config: SimulationConfig {
            target_dist: Length::new::<meter>(1000.0),
            num_sim: 1,
            dt: Time::new::<second>(1.0),
            max_steps: 2000,
            sample_rate: Time::new::<second>(5.0),
            result_path: None,
        },
        weather: Weather {
            temperature: ThermodynamicTemperature::new::<degree_celsius>(20.0),
            humidity: 50.0,
            solar_radiation: HeatFluxDensity::new::<watt_per_square_meter>(700.0),
            wind_speed: Velocity::new::<meter_per_second>(5.0),
            wind_azimuth: 90.0,
        },
        course: CourseProfile {
            distance: vec![
                Length::new::<meter>(0.0),
                Length::new::<meter>(100.0),
                Length::new::<meter>(200.0),
                Length::new::<meter>(300.0),
            ],
            grade: vec![0.0, 4.0, -3.0, 2.0],
            azimuth: vec![0.0, 90.0, 180.0, 270.0],
        },
        runners: vec![RunnerParams {
            runner_id: 1 as u32,
            f_max: Acceleration::new::<meter_per_second_squared>(3.0),
            e_init: AvailableEnergy::new::<joule_per_kilogram>(5000.0),
            tau: Time::new::<second>(5.0),
            sigma: SpecificPower::new::<watt_per_kilogram>(5.0),
            k: Frequency::new::<hertz>(0.1),
            gamma: Frequency::new::<hertz>(0.1),
            drag_coefficient: 1.0,
            frontal_area: Area::new::<square_meter>(0.5),
            mass: Mass::new::<kilogram>(70.0),
            rho: MassDensity::new::<kilogram_per_cubic_meter>(1.225),
            convection: HeatTransfer::new::<watt_per_square_meter_kelvin>(10.0),
            alpha: 0.5,
            psi: 0.01,
            const_v: Velocity::new::<meter_per_second>(3.5),
            const_f: Acceleration::new::<meter_per_second_squared>(0.0),
            pacing: PacingStrategy::Constant,
        }],
    };

    let sim = MonteCarloSimulation::new(input).expect("init failed");

    let mut grade_index = 0usize;
    let mut headwind_index = 0usize;

    let (theta_120, wind_120) = sim
        .lookup_course_conditions(
            Length::new::<meter>(120.0),
            &mut grade_index,
            &mut headwind_index,
        )
        .expect("lookup at 120m failed");
    assert_eq!(grade_index, 1);
    assert_eq!(headwind_index, 1);
    assert!((theta_120 - (0.04f64).atan()).abs() < 1e-12);
    assert!((wind_120.get::<meter_per_second>() + 5.0).abs() < 1e-12); // wind should be tailwind since azimuth are the same at 120m

    let (theta_190, wind_190) = sim
        .lookup_course_conditions(
            Length::new::<meter>(190.0),
            &mut grade_index,
            &mut headwind_index,
        )
        .expect("lookup at 190m failed");
    assert_eq!(grade_index, 2);
    assert_eq!(headwind_index, 2);
    assert!((theta_190 - (-0.03f64).atan()).abs() < 1e-12);
    assert!((wind_190.get::<meter_per_second>()).abs() < 1e-12); // wind should be 0 at 190m since it's a crosswind

    let (theta_290, wind_290) = sim
        .lookup_course_conditions(
            Length::new::<meter>(290.0),
            &mut grade_index,
            &mut headwind_index,
        )
        .expect("lookup at 290m failed");
    assert_eq!(grade_index, 3);
    assert_eq!(headwind_index, 3);
    assert!((theta_290 - (0.02f64).atan()).abs() < 1e-12);
    assert!((wind_290.get::<meter_per_second>() - 5.0).abs() < 1e-12); // wind should be headwind since azimuth are opposite at 290m
}