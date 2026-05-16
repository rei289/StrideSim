/*
Script performs a Monte Carlo simulation to determine all possible outcomes of running a marathon.
*/

use uom::si::f64::*;
use uom::si::frequency::hertz;
use uom::si::length::meter;
use uom::si::time::second;
use uom::si::velocity::meter_per_second;
use uom::si::acceleration::meter_per_second_squared;
use uom::si::available_energy::joule_per_kilogram;
use uom::si::specific_power::watt_per_kilogram;
use uom::si::heat_transfer::watt_per_square_meter_kelvin;
use uom::si::thermodynamic_temperature::degree_celsius;
use uom::si::heat_flux_density::watt_per_square_meter;

use crate::param_config::*;

use std::fs::File;
use std::sync::Arc;
use arrow_array::{
    ArrayRef, RecordBatch, UInt32Array, Float32Array, StringArray
};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::path::PathBuf;


const PARQUET_CHUNK_ROWS: usize = 50_000;


#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PacingStrategy {
    Constant,
    EvenEffort,
}

#[derive(Clone, Debug)]
pub struct SimulationConfig {
    pub target_dist: Length,            // target distance in meters   
    pub num_sim: usize,                 // number of monte carlo simulations to run (i.e. how many runners to simulate)
    pub dt: Time,                       // time step for the simulation (e.g. 0.1 means simulate every 0.1 seconds)
    pub max_steps: usize,               // maximum number of steps to simulate for each runner (to prevent infinite loops if they can't finish)
    pub sample_rate: Time,              // time in seconds to simulate before writing to parquet (e.g. 100 means write every 100 seconds)
    pub result_path: Option<String>,    // path to save results (e.g. parquet file)
}

#[derive(Clone, Debug)]
pub struct Weather {
    pub temperature: ThermodynamicTemperature,
    pub humidity: f64,                  // relative humidity as a percentage
    pub solar_radiation: HeatFluxDensity, // solar radiation in W/m^2
    pub wind_speed: Velocity,           // wind speed in m/s
    pub wind_azimuth: f64,              // wind direction in degrees from north (0-360)
}

#[derive(Clone, Debug)]
pub struct CourseProfile {
    pub distance: Vec<Length>,          // distance points for the course profile (m)
    pub grade: Vec<f64>,                // grade at each distance point as a percentage
    pub azimuth: Vec<f64>,              // azimuth at each distance point in degrees from north (0-360)
}

#[derive(Clone, Debug)]
pub struct RunnerParams {
    pub runner_id: u32,               // unique identifier for the runner
    pub f_max: Acceleration,            // maximum thrust (m/s^2)
    pub e_init: AvailableEnergy,        // initial energy (m^2/s^2)
    pub tau: Time,                      // resistance coefficient (s)
    pub sigma: SpecificPower,           // energy supply rate (m^2/s^3)
    pub k: Frequency,                   // fatigue coefficient for energy supply drop with velocity (1/s)
    pub gamma: Frequency,               // fatigue constant (1/s)
    pub drag_coefficient: f64,          // drag coefficient (dimensionless)
    pub frontal_area: Area,             // frontal area (m^2)
    pub mass: Mass,                     // mass (kg)
    pub rho: MassDensity,               // air density at sea level (kg/m^3)
    pub convection: HeatTransfer,       // convection heat transfer coefficient (W/m^2K)
    pub alpha: f64,                     // absorption coefficient for solar radiation (dimensionless)
    pub psi: f64,                       // weighting factor for the drop in aerobic power per temperature (dimensionless)
    pub const_v: Velocity,              // constant velocity for "constant" pacing strategy (m/s)
    pub const_f: Acceleration,          // constant acceleration for "even effort" pacing strategy (m/s^2)
    pub pacing: PacingStrategy,         // pacing strategy type
}

#[derive(Debug, Clone)]
pub struct SimulationInput {
    pub config: SimulationConfig,
    pub weather: Weather,
    pub course: CourseProfile,
    pub runners: Vec<RunnerParams>, // len must equal num_sim
}

#[derive(Debug, Clone)]
struct RunnerState {
    velocity: Velocity,
    energy: AvailableEnergy,
    distance: Length,
    time_elapsed: Time,
    finished_step: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ResultBatch {
    pub runner: Vec<u32>,
    pub time_s: Vec<f32>,
    pub velocity_mps: Vec<f32>,
    pub energy_jpkg: Vec<f32>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum SimError {
    InvalidConfig(&'static str),
    LengthMismatch(&'static str),
    InvalidValue(&'static str),
    EmptyCourse(&'static str),
}

pub struct MonteCarloSimulation {
    input: SimulationInput,
}

struct CourseIndex {
    grade_index: usize,
    azimuth_index: usize,
}

impl RunnerState {
    fn new(runner: &RunnerParams) -> Self {

        Self {
            velocity: Velocity::new::<meter_per_second>(1e-6),
            energy: runner.e_init,
            distance: Length::new::<meter>(0.0),
            time_elapsed: Time::new::<second>(0.0),
            finished_step: None,
        }
    }
}

impl ResultBatch {
    fn new() -> Self {
        Self {
            runner: Vec::with_capacity(PARQUET_CHUNK_ROWS),
            time_s: Vec::with_capacity(PARQUET_CHUNK_ROWS),
            velocity_mps: Vec::with_capacity(PARQUET_CHUNK_ROWS),
            energy_jpkg: Vec::with_capacity(PARQUET_CHUNK_ROWS),
        }
    }

    fn take_columns(&mut self) -> (Vec<u32>, Vec<f32>, Vec<f32>, Vec<f32>) {
        (
            std::mem::replace(&mut self.runner, Vec::with_capacity(PARQUET_CHUNK_ROWS)),
            std::mem::replace(&mut self.time_s, Vec::with_capacity(PARQUET_CHUNK_ROWS)),
            std::mem::replace(&mut self.velocity_mps, Vec::with_capacity(PARQUET_CHUNK_ROWS)),
            std::mem::replace(&mut self.energy_jpkg, Vec::with_capacity(PARQUET_CHUNK_ROWS)),
        )
    }

    // standard length method
    pub fn len(&self) -> usize {
        self.runner.len()
    }

    // always implement this if you have len()
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl MonteCarloSimulation {
    pub fn new(input: SimulationInput) -> Result<Self, SimError> {
        // simulation configuration validation
        if input.config.num_sim == 0 {
            return Err(SimError::InvalidConfig("num_sim must be > 0"));
        }
        if input.config.max_steps == 0 {
            return Err(SimError::InvalidConfig("max_steps must be > 0"));
        }
        if input.config.dt <= Time::new::<second>(0.0) {
            return Err(SimError::InvalidConfig("dt must be > 0"));
        }
        if input.config.target_dist <= Length::new::<meter>(0.0) {
            return Err(SimError::InvalidConfig("target_dist must be > 0"));
        }
        if input.runners.len() != input.config.num_sim {
            return Err(SimError::LengthMismatch("runners length must match num_sim"));
        }
        if input.config.sample_rate <= Time::new::<second>(0.0) {
            return Err(SimError::InvalidConfig("sample_rate must be > 0"));
        }

        // course profile validation
        let course_len = input.course.distance.len();
        if course_len == 0 {
            return Err(SimError::EmptyCourse("course distance must be non-empty"));
        }
        if input.course.grade.len() != course_len || input.course.azimuth.len() != course_len {
            return Err(SimError::LengthMismatch("course vectors must have equal lengths"));
        }
        
        // weather validation
        if input.weather.humidity < 0.0 || input.weather.humidity > 100.0 {
            return Err(SimError::InvalidValue("humidity must be between 0 and 100"));
        }
        if input.weather.solar_radiation < HeatFluxDensity::new::<watt_per_square_meter>(0.0) {
            return Err(SimError::InvalidValue("solar radiation must be non-negative"));
        }

        Ok(Self { input })
    }

    pub fn simulate(&mut self) -> Result<(), SimError> {
        /*
        Use to simulate the monte carlo process
         */
        let n = self.input.config.num_sim;
        let tmax = self.input.config.max_steps;

        // initialize parquet writer and row buffer for writing results in batches
        let mut writer = match self.input.config.result_path.as_deref() {
            Some(path) => Some(Self::make_parquet_writer(path)?),
            None => None,
        };
        let mut row_buffer = ResultBatch::new();

        let sample_rate_steps = (self.input.config.sample_rate.get::<second>() / self.input.config.dt.get::<second>()).round() as usize;
        
        // initialize state for each runner and perform the simulation
        for runner in 0..n {
            // calculate the new input parameters for this runner based on the weather conditions and their individual psi parameters, then perform the simulation for this runner until they finish or reach max steps
            self.init_runner(runner)?;
            let runner_param = &self.input.runners[runner];

            // keep indices across timesteps so nearest-point search is incremental.
            let mut course_index = CourseIndex {
                grade_index: 0,
                azimuth_index: 0,
            };
            
            let mut count = 0;
            // initialize the state for this runner
            let mut state = RunnerState::new(runner_param);

            // perform the simulation for this runner until they finish or reach max steps
            for step in 0..tmax {
                // check if runner has finished
                if state.distance >= self.input.config.target_dist {
                    state.finished_step = Some(step);
                    break;
                }

                if state.velocity <= Velocity::new::<meter_per_second>(1e-8)
                    && state.energy <= AvailableEnergy::new::<joule_per_kilogram>(1e-8)
                {
                    state.finished_step = Some(step);
                    break;
                }
                
                // if we have a parquet writer, write the current state to the row buffer at the specified sample rate
                if writer.is_some() && count == sample_rate_steps {
                // determine if we should write to parquet based on sample_rate
                    row_buffer.runner.push(runner as u32);
                    row_buffer.time_s.push(state.time_elapsed.get::<second>() as f32);
                    row_buffer.velocity_mps.push(state.velocity.get::<meter_per_second>() as f32);
                    row_buffer.energy_jpkg.push(state.energy.get::<joule_per_kilogram>() as f32);
                    count = 0; // reset the counter
                }

                if let Some(w) = writer.as_mut() {
                    if row_buffer.len() >= PARQUET_CHUNK_ROWS {
                        Self::flush_parquet_rows( w, &mut row_buffer)?;
                    }
                }

                count += 1;
                self.step_runner(runner_param, &mut state, &mut course_index)?;
            }
        }


        if let Some(w) = writer.as_mut() {
            if !row_buffer.is_empty() {
                Self::flush_parquet_rows( w, &mut row_buffer)?;
            }
        }

        if let Some(w) = writer {
            Self::close_parquet_writer(w)?;
        }

        if let Some(path) = self.input.config.result_path.as_deref() {
            self.write_runner_params_parquet(path)?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn simulate_collect(&mut self) -> Result<ResultBatch, SimError> {
        /*
        Use to simulate the monte carlo process and collect results in memory instead of writing to parquet
         */
        let n = self.input.config.num_sim;
        let tmax = self.input.config.max_steps;

        let mut results = ResultBatch::new();
        let sample_rate_steps = (self.input.config.sample_rate.get::<second>() / self.input.config.dt.get::<second>()).round() as usize;

        // initialize state for each runner and perform the simulation
        for runner in 0..n {
            // initialize current index for course profile lookup based on current distance
            let mut course_index = CourseIndex {
                grade_index: 0,
                azimuth_index: 0,
            };
            // calculate the new input parameters for this runner based on the weather conditions and their individual psi parameters, then perform the simulation for this runner until they finish or reach max steps
            self.init_runner(runner)?;
            let runner_param = &self.input.runners[runner];

            // initialize the state for this runner
            let mut state = RunnerState::new(runner_param);

            let mut count = 0;
            // perform the simulation for this runner until they finish or reach max steps
            for step in 0..tmax {
                // check if runner has finished
                if state.distance >= self.input.config.target_dist {
                    state.finished_step = Some(step);
                    break;
                }

                if state.velocity <= Velocity::new::<meter_per_second>(1e-8)
                    && state.energy <= AvailableEnergy::new::<joule_per_kilogram>(1e-8)
                {
                    state.finished_step = Some(step);
                    break;
                }

                if count == sample_rate_steps {
                    results.runner.push(runner as u32);
                    results.time_s.push(state.time_elapsed.get::<second>() as f32);
                    results.velocity_mps.push(state.velocity.get::<meter_per_second>() as f32);
                    results.energy_jpkg.push(state.energy.get::<joule_per_kilogram>() as f32);
                    count = 0; // reset the counter
                }
                self.step_runner(runner_param, &mut state, &mut course_index)?;
                count += 1;
            }
        }

        Ok(results)
    }



    fn init_runner(
        &mut self,
        runner_idx: usize,
    ) -> Result<(), SimError> {
        // first update the sigma values for all runners based on the weather conditions and their individual psi parameters
        let wbgt_c = self.get_wbgt(&self.input.runners[runner_idx])?.get::<degree_celsius>();
        // let wbgt_c = self.get_wbgt(runner_param)?.get::<degree_celsius>();
        let ref_temp_c = reference_temp_c();
        let temp_diff = (wbgt_c - ref_temp_c).max(0.0);
        let psi = self.input.runners[runner_idx].psi;
        self.input.runners[runner_idx].sigma *= 1.0 - psi * temp_diff;
        
        
        // also update the k value which is just the 2 times of gamma
        self.input.runners[runner_idx].k = Frequency::new::<hertz>(2.0 * self.input.runners[runner_idx].gamma.get::<hertz>());
        
        // if pacing strategy is even effort, calculate the constant acceleration
        if self.input.runners[runner_idx].pacing == PacingStrategy::EvenEffort {
            let (rho, cd, area, mass, const_v, tau) = {
                let r = &self.input.runners[runner_idx];
                (r.rho, r.drag_coefficient, r.frontal_area, r.mass, r.const_v, r.tau)
            };
            
            let f_resist = 0.5 * rho * cd * area * const_v * const_v / mass;
            let const_f = f_resist + (const_v / tau);
            
            self.input.runners[runner_idx].const_f = const_f;
        }

        Ok(())
    }

    fn parquet_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("runner_id", DataType::UInt32, false),
            Field::new("time_s", DataType::Float32, false),
            Field::new("velocity_mps", DataType::Float32, false),
            Field::new("energy_jpkg", DataType::Float32, false),
        ]))
    }

    fn make_parquet_writer(path: &str) -> Result<ArrowWriter<File>, SimError> {
        std::fs::create_dir_all(path)
            .map_err(|_| SimError::InvalidValue("failed to create result directory"))?;
        let out_path = PathBuf::from(path).join("simulation_results.parquet");

        let file = File::create(out_path)
            .map_err(|_| SimError::InvalidValue("failed to create simulation result parquet file"))?;
        let props = WriterProperties::builder()
            .set_dictionary_enabled(true)
            .build();
        ArrowWriter::try_new(file, Self::parquet_schema(), Some(props))
            .map_err(|_| SimError::InvalidValue("failed to create simulation result parquet writer"))
    }

    fn flush_parquet_rows(
        writer: &mut ArrowWriter<File>,
        results: &mut ResultBatch,
        // rows: &mut Vec<ResultRow>,
    ) -> Result<(), SimError> {
        let (runner_col, time_col, velocity_col, energy_col) = results.take_columns();

        let runner_array = UInt32Array::from(runner_col);
        let time_array = Float32Array::from(time_col);
        let velocity_array = Float32Array::from(velocity_col);
        let energy_array = Float32Array::from(energy_col);
        let batch = RecordBatch::try_new(
            Self::parquet_schema(),
            vec![
                Arc::new(runner_array) as ArrayRef,
                Arc::new(time_array) as ArrayRef,
                Arc::new(velocity_array) as ArrayRef,
                Arc::new(energy_array) as ArrayRef,
            ],
        ).map_err(|_| SimError::InvalidValue("failed to create record batch"))?;

        writer.write(&batch)
            .map_err(|_| SimError::InvalidValue("failed to write record batch to parquet"))?;

        // results.clear();
        Ok(())
    }

    fn close_parquet_writer(writer: ArrowWriter<File>) -> Result<(), SimError> {
        writer.close()
            .map_err(|_| SimError::InvalidValue("failed to close parquet writer"))?;
        Ok(())
    }

    fn runner_params_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("runner_id", DataType::UInt32, false),
            Field::new("f_max_mps2", DataType::Float32, false),
            Field::new("e_init_jpkg", DataType::Float32, false),
            Field::new("tau_s", DataType::Float32, false),
            Field::new("sigma_wpkg", DataType::Float32, false),
            Field::new("gamma_hz", DataType::Float32, false),
            Field::new("drag_coefficient", DataType::Float32, false),
            Field::new("frontal_area_m2", DataType::Float32, false),
            Field::new("mass_kg", DataType::Float32, false),
            Field::new("rho_kgpm3", DataType::Float32, false),
            Field::new("convection_wpm2k", DataType::Float32, false),
            Field::new("alpha", DataType::Float32, false),
            Field::new("psi", DataType::Float32, false),
            Field::new("const_v_mps", DataType::Float32, false),
            Field::new("pacing", DataType::Utf8, false),
        ]))
    }

    fn write_runner_params_parquet(&self, path: &str) -> Result<(), SimError> {
        std::fs::create_dir_all(path)
            .map_err(|_| SimError::InvalidValue("failed to create result directory"))?;

        let out_path = PathBuf::from(path).join("runner_params.parquet");
        let file = File::create(&out_path)
            .map_err(|_| SimError::InvalidValue("failed to create runner params parquet"))?;

        let mut writer = ArrowWriter::try_new(file, Self::runner_params_schema(), Some(
            WriterProperties::builder().set_dictionary_enabled(true).build()
        ))
        .map_err(|_| SimError::InvalidValue("failed to create runner params writer"))?;

        let mut runner_id = Vec::with_capacity(self.input.runners.len());
        let mut f_max = Vec::with_capacity(self.input.runners.len());
        let mut e_init = Vec::with_capacity(self.input.runners.len());
        let mut tau = Vec::with_capacity(self.input.runners.len());
        let mut sigma = Vec::with_capacity(self.input.runners.len());
        let mut gamma = Vec::with_capacity(self.input.runners.len());
        let mut drag = Vec::with_capacity(self.input.runners.len());
        let mut area = Vec::with_capacity(self.input.runners.len());
        let mut mass = Vec::with_capacity(self.input.runners.len());
        let mut rho = Vec::with_capacity(self.input.runners.len());
        let mut convection = Vec::with_capacity(self.input.runners.len());
        let mut alpha = Vec::with_capacity(self.input.runners.len());
        let mut psi = Vec::with_capacity(self.input.runners.len());
        let mut const_v = Vec::with_capacity(self.input.runners.len());
        let mut pacing = Vec::with_capacity(self.input.runners.len());

        for r in &self.input.runners {
            runner_id.push(r.runner_id);
            f_max.push(r.f_max.get::<meter_per_second_squared>() as f32);
            e_init.push(r.e_init.get::<joule_per_kilogram>() as f32);
            tau.push(r.tau.get::<second>() as f32);
            sigma.push(r.sigma.get::<watt_per_kilogram>() as f32);
            gamma.push(r.gamma.get::<hertz>() as f32);
            drag.push(r.drag_coefficient as f32);
            area.push(r.frontal_area.get::<uom::si::area::square_meter>() as f32);
            mass.push(r.mass.get::<uom::si::mass::kilogram>() as f32);
            rho.push(r.rho.get::<uom::si::mass_density::kilogram_per_cubic_meter>() as f32);
            convection.push(r.convection.get::<watt_per_square_meter_kelvin>() as f32);
            alpha.push(r.alpha as f32);
            psi.push(r.psi as f32);
            const_v.push(r.const_v.get::<meter_per_second>() as f32);
            pacing.push(match r.pacing {
                PacingStrategy::Constant => "constant",
                PacingStrategy::EvenEffort => "even_effort",
            });
        }

        let batch = RecordBatch::try_new(
            Self::runner_params_schema(),
            vec![
                Arc::new(UInt32Array::from(runner_id)) as ArrayRef,
                Arc::new(Float32Array::from(f_max)) as ArrayRef,
                Arc::new(Float32Array::from(e_init)) as ArrayRef,
                Arc::new(Float32Array::from(tau)) as ArrayRef,
                Arc::new(Float32Array::from(sigma)) as ArrayRef,
                Arc::new(Float32Array::from(gamma)) as ArrayRef,
                Arc::new(Float32Array::from(drag)) as ArrayRef,
                Arc::new(Float32Array::from(area)) as ArrayRef,
                Arc::new(Float32Array::from(mass)) as ArrayRef,
                Arc::new(Float32Array::from(rho)) as ArrayRef,
                Arc::new(Float32Array::from(convection)) as ArrayRef,
                Arc::new(Float32Array::from(alpha)) as ArrayRef,
                Arc::new(Float32Array::from(psi)) as ArrayRef,
                Arc::new(Float32Array::from(const_v)) as ArrayRef,
                Arc::new(StringArray::from(pacing)) as ArrayRef,
            ],
        )
        .map_err(|_| SimError::InvalidValue("failed to create runner params batch"))?;

        writer.write(&batch)
            .map_err(|_| SimError::InvalidValue("failed to write runner params batch"))?;
        writer.close()
            .map_err(|_| SimError::InvalidValue("failed to close runner params writer"))?;
        Ok(())
    }


    fn step_runner(&self, runner_params: &RunnerParams, state: &mut RunnerState, course_index: &mut CourseIndex) -> Result<(), SimError> {
        /*
        Use to perform one time step update for a given runner, calculating its current location and all the physics
         */
        let dt = self.input.config.dt;

        let (grade, headwind) = self.lookup_course_conditions(state.distance, &mut course_index.grade_index, &mut course_index.azimuth_index)?;
        
        // let grade = self.get_grade(state.distance, &mut course_index.grade_index)?;
        // let headwind = self.get_headwind(state.distance, &mut course_index.headwind_index)?;

        // determine the desired acceleration based on the pacing strategy and current state
        let f_desired = match runner_params.pacing {
            PacingStrategy::Constant => (runner_params.const_v - state.velocity)/dt,
            PacingStrategy::EvenEffort => runner_params.const_f,
        };

        // where the math model calculates monte carlo
        self.math_model(runner_params, state, f_desired, grade, headwind)?;
        

        Ok(())
    }

    fn math_model(&self,  runner_params: &RunnerParams, state: &mut RunnerState, f_desired: Acceleration, theta: f64, headwind: Velocity) -> Result<(), SimError> {
        /*
        Model the physics of the runner's movement for one time step, updating the velocity, energy, and distance in the state.
         */
        // define local variables for readability
        let (rho, cd, area, mass, e_init, tau, sigma, k, f_max) = {
            let r = runner_params;
            (r.rho, r.drag_coefficient, r.frontal_area, r.mass, r.e_init, r.tau, r.sigma, r.k, r.f_max)
        };

        let dt = self.input.config.dt;

        let current_v = state.velocity;
        let current_e = state.energy;
        let current_t = state.time_elapsed;
        let current_d = state.distance;

        // firt calculate all the resistive forces
        let v_rel = current_v + headwind; // relative velocity for drag calculation
        let f_resistance = Acceleration::new::<meter_per_second_squared>(gravity_mps2())*theta.sin()
                        + (0.5*rho*cd*area*v_rel*v_rel)/mass;

        // calculate the actual force applied by the runner, which is limited by the maximum thrust
        let f_possible = f_desired.min(f_max);

        // check if the runner has enough energy to apply the actual force
        let f_final =  if current_e > AvailableEnergy::new::<joule_per_kilogram>(0.0) {
            f_possible
        } else {
            // if not enough energy, calculate the maximum possible force based on remaining energy and power supply rate
            (sigma - (k*current_v*current_v*current_t)/tau) / current_v
        };

        // calculate the change in velocity and energy based on the actual force applied
        let dv = f_final - f_resistance - (1.0/tau) * current_v;

        let de = if current_e > AvailableEnergy::new::<joule_per_kilogram>(0.0) {
            sigma - (f_final + f_resistance)*current_v - (k*current_v*current_v*current_t)/tau
        } else {
            SpecificPower::new::<watt_per_kilogram>(0.0)
        };

        // update velocity and energy for the next iteration
        let v_next = (current_v + dv*dt).max(Velocity::new::<meter_per_second>(0.0)); // velocity cannot be negative
        let e_next = if current_e + de*dt < AvailableEnergy::new::<joule_per_kilogram>(0.0) {
            AvailableEnergy::new::<joule_per_kilogram>(0.0)
        } else if current_e + de*dt > e_init {
            e_init
        } 
        else {
            current_e + de*dt
        };

        state.velocity = v_next;
        state.energy = e_next;

        // update distance and time
        state.distance = current_d + v_next * dt;
        state.time_elapsed = current_t + dt;

        Ok(())
    }

    fn get_grade(&self, distance: Length, course_index: &mut usize) -> Result<f64, SimError> {
        /*
        Helper function to get the grade at a given distance based on the course profile.
         */
        let course = &self.input.course;
        let mut current_diff = (distance - course.distance[*course_index]).abs().get::<meter>();
        for i in (*course_index + 1)..course.distance.len() {
            // get the absolute difference between the current distance and the course distance at index i
            let diff = (distance - course.distance[i]).abs().get::<meter>();
            if diff < current_diff {
                current_diff = diff;
                *course_index = i;
            } else {
                break; // since the course distance is sorted, we can break once the difference starts increasing
            }
        }
        
        // convert grade from percent to angle (radians) for physics calculations
        let decimal_grade: f64 = course.grade[*course_index] / 100.0;
        
        Ok(decimal_grade.atan())
    }

    fn get_headwind(&self, distance: Length, azimuth_index: &mut usize) -> Result<Velocity, SimError> {
        /*
        Helper function to get the headwind at a given distance based on the course profile.
         */
        let course = &self.input.course;
        let mut current_diff = (distance - course.distance[*azimuth_index]).abs().get::<meter>();

        for i in (*azimuth_index + 1)..course.distance.len() {
            // get the absolute difference between the current distance and the course distance at index i
            let diff = (distance - course.distance[i]).abs().get::<meter>();
            if diff < current_diff {
                current_diff = diff;
                *azimuth_index = i;
            } else {
                break; // since the course distance is sorted, we can break once the difference starts increasing
            }
        }

            // calculate the headwind based on the course azimuth and the wind conditions
        let course_azimuth = course.azimuth[*azimuth_index];
        let wind_azimuth = self.input.weather.wind_azimuth;
        let wind_speed = self.input.weather.wind_speed;
        let relative_azimuth = (course_azimuth - wind_azimuth).to_radians();
        let headwind_speed = - wind_speed * relative_azimuth.cos(); // positive if headwind, negative if tailwind

        Ok(headwind_speed)

    }

    pub fn lookup_course_conditions(
        &self,
        distance: Length,
        grade_index: &mut usize,
        azimuth_index: &mut usize,
    ) -> Result<(f64, Velocity), SimError> {
        let grade = self.get_grade(distance, grade_index)?;
        let headwind = self.get_headwind(distance, azimuth_index)?;
        Ok((grade, headwind))
    }

    fn get_wbgt(&self, runner: &RunnerParams) -> Result<ThermodynamicTemperature, SimError> {
        /*
        Helper function to calculate the Wet Bulb Globe Temperature (WBGT) based on the weather conditions.
         */
        let weather = &self.input.weather;

        // converrt units to dimensionless for empirical formula
        let temp_c = weather.temperature.get::<degree_celsius>();
        let humidity_percent = weather.humidity; // already in percentage
        let solar_w_m2 = weather.solar_radiation.get::<watt_per_square_meter>();
        let conv_w_m2k = runner.convection.get::<watt_per_square_meter_kelvin>();
        let alpha = runner.alpha;

        let temp_w = temp_c * (0.151977*((humidity_percent + 8.313659).powf(0.5))).atan()
            + (temp_c + humidity_percent).atan()
            - (humidity_percent - 1.676331).atan()
            + 0.00391838*(humidity_percent).powf(1.5) * (0.023101 * humidity_percent).atan()
            - 4.686035;
        let temp_g = temp_c + solar_w_m2 / (conv_w_m2k * alpha);
        let wbgt = 0.7*temp_w + 0.2*temp_g + 0.1*temp_c;

        Ok(ThermodynamicTemperature::new::<degree_celsius>(wbgt))

    }

}

