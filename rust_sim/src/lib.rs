pub mod monte_carlo_simulation;
pub mod param_config;

use monte_carlo_simulation::{
	CourseProfile as CoreCourseProfile, MonteCarloSimulation, PacingStrategy,
	RunnerParams as CoreRunnerParams, SimulationConfig as CoreSimulationConfig, SimulationInput,
	Weather as CoreWeather
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList};

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

#[pyclass]
#[derive(Clone)]
struct SimulationConfig {
	#[pyo3(get, set)]
	target_dist: f64,
	#[pyo3(get, set)]
	num_sim: usize,
	#[pyo3(get, set)]
	dt: f64,
	#[pyo3(get, set)]
	max_steps: usize,
	#[pyo3(get, set)]
	sample_rate: f64,
	#[pyo3(get, set)]
	result_path: Option<String>,
}

#[pymethods]
impl SimulationConfig {
	#[new]
	#[pyo3(signature = (target_dist, num_sim, dt, max_steps, sample_rate, result_path))]
	fn new(target_dist: f64, num_sim: usize, dt: f64, max_steps: usize, sample_rate: f64, result_path: Option<String>) -> Self {
		Self {
			target_dist,
			num_sim,
			dt,
			max_steps,
			sample_rate,
			result_path,
		}
	}
}

impl SimulationConfig {
	fn to_core(&self) -> CoreSimulationConfig {
		CoreSimulationConfig {
			target_dist: Length::new::<meter>(self.target_dist),
			num_sim: self.num_sim,
			dt: Time::new::<second>(self.dt),
			max_steps: self.max_steps,
			sample_rate: Time::new::<second>(self.sample_rate),
			result_path: self.result_path.clone(),
		}
	}
}

#[pyclass]
#[derive(Clone)]
struct Weather {
	#[pyo3(get, set)]
	temperature: Option<f64>,
	#[pyo3(get, set)]
	humidity: Option<f64>,
	#[pyo3(get, set)]
	solar_radiation: Option<f64>,
	#[pyo3(get, set)]
	wind_speed: Option<f64>,
	#[pyo3(get, set)]
	wind_azimuth: Option<f64>,
}

#[pymethods]
impl Weather {
	#[new]
	fn new(temperature: Option<f64>, humidity: Option<f64>, solar_radiation: Option<f64>, wind_speed: Option<f64>, wind_azimuth: Option<f64>) -> Self {
		Self {
			temperature,
			humidity,
			solar_radiation,
			wind_speed,
			wind_azimuth,
		}
	}
}

impl Weather {
	fn to_core(&self) -> CoreWeather {
		CoreWeather {
			temperature: ThermodynamicTemperature::new::<degree_celsius>(self.temperature.unwrap_or(20.0)),
			humidity: self.humidity.unwrap_or(50.0),
			solar_radiation: HeatFluxDensity::new::<watt_per_square_meter>(self.solar_radiation.unwrap_or(50.0)),
			wind_speed: Velocity::new::<meter_per_second>(self.wind_speed.unwrap_or(0.0)),
			wind_azimuth: self.wind_azimuth.unwrap_or(0.0),
		}
	}
}

#[pyclass]
#[derive(Clone)]
struct CourseProfile {
	#[pyo3(get, set)]
	distance: Option<Vec<f64>>,
	#[pyo3(get, set)]
	grade: Option<Vec<f64>>,
	#[pyo3(get, set)]
	azimuth: Option<Vec<f64>>,
}

#[pymethods]
impl CourseProfile {
	#[new]
	fn new(distance: Option<Vec<f64>>, grade: Option<Vec<f64>>, azimuth: Option<Vec<f64>>) -> Self {
		Self {
			distance,
			grade,
			azimuth,
		}
	}
}

impl CourseProfile {
	fn to_core(&self) -> CoreCourseProfile  {
        let distance_vals = self
            .distance
            .clone()
            .unwrap_or_else(|| vec![0.0]);

        let grade_vals = self
            .grade
            .clone()
            .unwrap_or_else(|| vec![0.0]);

        let azimuth_vals = self
            .azimuth
            .clone()
            .unwrap_or_else(|| vec![0.0]);

        CoreCourseProfile {
            distance: distance_vals
                .into_iter()
                .map(|d| Length::new::<meter>(d))
                .collect(),
            grade: grade_vals,
            azimuth: azimuth_vals
        }
    }
}

#[pyclass]
#[derive(Clone)]
struct RunnerParams {
	#[pyo3(get, set)]
	runner_id: u32,
	#[pyo3(get, set)]
	f_max: f64,
	#[pyo3(get, set)]
	e_init: f64,
	#[pyo3(get, set)]
	tau: f64,
	#[pyo3(get, set)]
	sigma: f64,
	#[pyo3(get, set)]
	gamma: f64,
	#[pyo3(get, set)]
	drag_coefficient: f64,
	#[pyo3(get, set)]
	frontal_area: f64,
	#[pyo3(get, set)]
	mass: f64,
	#[pyo3(get, set)]
	rho: f64,
	#[pyo3(get, set)]
	convection: f64,
	#[pyo3(get, set)]
	alpha: f64,
	#[pyo3(get, set)]
	psi: f64,
	#[pyo3(get, set)]
	const_v: f64,
	#[pyo3(get, set)]
	pacing: String,
}

#[pymethods]
impl RunnerParams {
	#[new]
	#[allow(clippy::too_many_arguments)]
	#[pyo3(signature = (
		runner_id,
		f_max,
		e_init,
		tau,
		sigma,
		gamma,
		drag_coefficient,
		frontal_area,
		mass,
		rho,
		convection,
		alpha,
		psi,
		const_v,
		pacing="constant".to_string()
	))]
	fn new(
		runner_id: u32,
		f_max: f64,
		e_init: f64,
		tau: f64,
		sigma: f64,
		gamma: f64,
		drag_coefficient: f64,
		frontal_area: f64,
		mass: f64,
		rho: f64,
		convection: f64,
		alpha: f64,
		psi: f64,
		const_v: f64,
		pacing: String,
	) -> Self {
		Self {
			runner_id,
			f_max,
			e_init,
			tau,
			sigma,
			gamma,
			drag_coefficient,
			frontal_area,
			mass,
			rho,
			convection,
			alpha,
			psi,
			const_v,
			pacing,
		}
	}
}

impl RunnerParams {
	fn to_core(&self) -> PyResult<CoreRunnerParams> {
		let pacing_normalized = self.pacing.trim().to_ascii_lowercase();
		let pacing = match pacing_normalized.as_str() {
			"constant" | "constant_velocity" | "constant velocity" => PacingStrategy::Constant,
			"eveneffort" | "even_effort" | "even effort" => PacingStrategy::EvenEffort,
			_ => {
				return Err(PyValueError::new_err(
					"Invalid pacing. Use 'constant' or 'even_effort'",
				))
			}
		};

		Ok(CoreRunnerParams {
			runner_id: self.runner_id,
			f_max: Acceleration::new::<meter_per_second_squared>(self.f_max),
			e_init: AvailableEnergy::new::<joule_per_kilogram>(self.e_init),
			tau: Time::new::<second>(self.tau),
			sigma: SpecificPower::new::<watt_per_kilogram>(self.sigma),
            k: Frequency::new::<hertz>(0.0), // placeholder, will be set in MonteCarloSimulation::new based on pacing strategy
			gamma: Frequency::new::<hertz>(self.gamma),
			drag_coefficient: self.drag_coefficient,
			frontal_area: Area::new::<square_meter>(self.frontal_area),
			mass: Mass::new::<kilogram>(self.mass),
			rho: MassDensity::new::<kilogram_per_cubic_meter>(self.rho),
			convection: HeatTransfer::new::<watt_per_square_meter_kelvin>(self.convection),
			alpha: self.alpha,
			psi: self.psi,
			const_v: Velocity::new::<meter_per_second>(self.const_v),
            const_f: Acceleration::new::<meter_per_second_squared>(0.0), // placeholder, will be set in MonteCarloSimulation::new based on pacing strategy
			pacing,
		})
	}
}

#[pyfunction]
fn module_info() -> &'static str {
	"Initialize SimulationConfig/Weather/CourseProfile/RunnerParams in Python, then call run_simulation(config, weather, course, runners)"
}

#[pyfunction]
fn run_simulation(
	_py: Python<'_>,
	config: PyRef<'_, SimulationConfig>,
	weather: PyRef<'_, Weather>,
	course: PyRef<'_, CourseProfile>,
	runners: &Bound<'_, PyList>,
) -> PyResult<()> {
// ) -> PyResult<PyObject> {
	let mut core_runners = Vec::with_capacity(runners.len());
	for item in runners.iter() {
		let runner: PyRef<'_, RunnerParams> = item.extract()?;
		core_runners.push(runner.to_core()?);
	}

	let input = SimulationInput {
		config: config.to_core(),
		weather: weather.to_core(),
		course: course.to_core(),
		runners: core_runners,
	};

	let mut sim = MonteCarloSimulation::new(input)
		.map_err(|err| PyValueError::new_err(format!("init error: {:?}", err)))?;

	let _ = sim
		.simulate()
		.map_err(|err| PyValueError::new_err(format!("simulate error: {:?}", err)))?;

	Ok(())
}

#[pyfunction]
fn run_simulation_collect(
	_py: Python<'_>,
	config: PyRef<'_, SimulationConfig>,
	weather: PyRef<'_, Weather>,
	course: PyRef<'_, CourseProfile>,
	runners: &Bound<'_, PyList>,
) -> PyResult<(Vec<u32>, Vec<f32>, Vec<f32>, Vec<f32>)> {
		let mut core_runners = Vec::with_capacity(runners.len());
	for item in runners.iter() {
		let runner: PyRef<'_, RunnerParams> = item.extract()?;
		core_runners.push(runner.to_core()?);
	}

	let input = SimulationInput {
		config: config.to_core(),
		weather: weather.to_core(),
		course: course.to_core(),
		runners: core_runners,
	};

	let mut sim = MonteCarloSimulation::new(input)
		.map_err(|err| PyValueError::new_err(format!("init error: {:?}", err)))?;

	let results = sim
		.simulate_collect()
		.map_err(|err| PyValueError::new_err(format!("simulate error: {:?}", err)))?;

	Ok((results.runner, results.time_s, results.velocity_mps, results.energy_jpkg))
}

#[pymodule]
fn stride_sim_rust(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
	m.add_class::<SimulationConfig>()?;
	m.add_class::<Weather>()?;
	m.add_class::<CourseProfile>()?;
	m.add_class::<RunnerParams>()?;
	m.add_function(wrap_pyfunction!(module_info, m)?)?;
	m.add_function(wrap_pyfunction!(run_simulation, m)?)?;
	m.add_function(wrap_pyfunction!(run_simulation_collect, m)?)?;
	Ok(())
}
