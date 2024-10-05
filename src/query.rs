use windows_sys::{
    Win32::{
        Foundation::{ERROR_SUCCESS, HWND, SYSTEMTIME},
        System::{
            Performance::{
                PdhAddCounterW, PdhBrowseCountersW, PdhCloseQuery, PdhCollectQueryData,
                PdhGetFormattedCounterArrayW, PdhOpenQueryW, PDH_BROWSE_DLG_CONFIG_W,
                PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_MAX_COUNTER_PATH, PDH_MORE_DATA,
            },
            SystemInformation::GetLocalTime,
        },
        UI::{
            WindowsAndMessaging::{SendMessageW, WM_USER},
            Controls::Dialogs::{OPENFILENAMEW, GetSaveFileNameW},
        },
    },
    w,
};

use random_color::{Luminosity, RandomColor};

use std::{
    cmp, collections, fs,
    io::Write,
    iter, mem, ptr,
    sync::mpsc::{self, TryRecvError},
    thread,
    time::{self, Duration},
    env,
    path,
};

use super::menu;

pub const WM_UPDATE_QUERY: u32 = WM_USER + 1;
pub const SAMPLE_COUNT: usize = 20;

pub struct CounterV2 {
    pub path: Vec<u16>,
    hcounter: isize,
    data: collections::VecDeque<f64>,

    pub interpolated_curves: Vec<InterpolatedCurve>,

    nb_instance: usize,
    instance_names: Vec<String>,

    pub instance_colors: Vec<[u8; 4]>,

    pub max: [f64; 2],
    pub avg: [f64; 2],
}

impl CounterV2 {
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new(hwnd: HWND, hquery: isize, path: Option<Vec<u16>>) -> Option<Self> {
        let path = path.unwrap_or_else(|| {
            let mut path_buffer =
                Vec::from_iter(iter::repeat(0_u16).take(PDH_MAX_COUNTER_PATH as usize));

            let mut bw_config: PDH_BROWSE_DLG_CONFIG_W = mem::zeroed();
            bw_config._bitfield = 0b0000_0000_0000_0000_0000_0001_0001_0111;
            bw_config.hWndOwner = hwnd;
            bw_config.szReturnPathBuffer = path_buffer.as_mut_ptr();
            bw_config.cchReturnPathLength = PDH_MAX_COUNTER_PATH;
            bw_config.CallBackStatus = ERROR_SUCCESS as _;
            bw_config.szDialogBoxCaption = &mut 0;

            if PdhBrowseCountersW(&bw_config) != ERROR_SUCCESS || path_buffer[0] == 0 {
                return Vec::new();
            };

            path_buffer.retain(|&c| c != 0);
            path_buffer.push(0);

            path_buffer
        });

        let mut hcounter = 0;
        if PdhAddCounterW(hquery, path.as_ptr(), 0, &mut hcounter) != ERROR_SUCCESS {
            return None;
        };

        Some(Self {
            path,
            hcounter,
            data: collections::VecDeque::new(),

            nb_instance: 0,
            instance_names: Vec::new(),

            interpolated_curves: Vec::new(),

            instance_colors: Vec::new(),

            max: [0.0, 0.0],
            avg: [0.0, 0.0],
        })
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn update(
        &mut self,
    ) -> (
        &Vec<u16>,
        Option<impl iter::Iterator<Item = (&f64, &String)>>,
    ) {
        let mut buffer_size = 0;
        let mut item_count = 0;

        if PdhGetFormattedCounterArrayW(
            self.hcounter,
            PDH_FMT_DOUBLE,
            &mut buffer_size,
            &mut item_count,
            ptr::null_mut(),
        ) != PDH_MORE_DATA
        {
            return (&self.path, None);
        };

        let mut item_buffer = Vec::from_iter(
            iter::repeat(mem::zeroed::<PDH_FMT_COUNTERVALUE_ITEM_W>())
                .take(buffer_size as usize / mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>() + 1),
        );

        if PdhGetFormattedCounterArrayW(
            self.hcounter,
            PDH_FMT_DOUBLE,
            &mut buffer_size,
            &mut item_count,
            item_buffer.as_mut_ptr(),
        ) != ERROR_SUCCESS
        {
            return (&self.path, None);
        };

        // we now have for sure a sample

        if self.nb_instance != item_count as usize {
            self.nb_instance = item_count as usize;

            self.data =
                collections::VecDeque::from_iter(iter::repeat(0.0).take(self.nb_instance * 20));
            self.interpolated_curves = Vec::new();

            self.instance_colors = (0..self.nb_instance)
                .map(|_| {
                    let tmp = RandomColor::new()
                        .luminosity(Luminosity::Light)
                        .to_rgb_array();

                    [tmp[0], tmp[1], tmp[2], 255]
                })
                .collect();

            self.instance_names = item_buffer
                .iter()
                .take(self.nb_instance)
                .map(|item| {
                    let mut curr = item.szName;
                    String::from_utf16(
                        iter::repeat_with(|| {
                            let tmp = *curr;
                            curr = curr.add(1);
                            tmp
                        })
                        .take_while(|tmp| tmp != &0)
                        .collect::<Vec<u16>>()
                        .as_slice(),
                    )
                    .unwrap()
                })
                .collect();
        }

        self.data.extend(
            item_buffer
                .iter()
                .take(self.nb_instance)
                .map(|item| item.FmtValue.Anonymous.doubleValue),
        );
        drop(self.data.drain(
            0..cmp::max(
                0,
                self.data.len() as isize - (self.nb_instance * SAMPLE_COUNT) as isize,
            ) as usize,
        ));

        self.interpolated_curves = (0..self.nb_instance)
            .map(|of| {
                InterpolatedCurve::new(
                    self.data
                        .iter()
                        .skip(of)
                        .step_by(self.nb_instance)
                        .map(|val| *val as f32),
                )
            })
            .collect::<Vec<_>>();

        let (mut max, mut tmp_max, mut avg) = (f64::MIN, f64::MIN, 0.0);
        for of in (0..self.data.len()).step_by(self.nb_instance) {
            for tmp_value in self.data.range(of..(of + self.nb_instance)) {
                if *tmp_value > tmp_max {
                    tmp_max = *tmp_value;
                }
            }

            if tmp_max > max {
                max = tmp_max;
            }

            avg += tmp_max / (self.data.len() / self.nb_instance) as f64;
            tmp_max = f64::MIN;
        }
        self.max[0] = self.max[1];
        self.max[1] = max;
        self.avg[0] = self.avg[1];
        self.avg[1] = avg;

        (
            &self.path,
            Some(
                self.data
                    .range((self.data.len() - self.nb_instance)..self.data.len())
                    .zip(self.instance_names.iter()),
            ),
        )
    }

    #[allow(clippy::missing_safety_doc)]
    pub fn get_data_by_instance(
        &self,
    ) -> Option<impl iter::Iterator<Item = impl iter::Iterator<Item = &f64>>> {
        if !self.data.is_empty() {
            Some(
                (0..self.nb_instance)
                    .map(|offset| self.data.iter().skip(offset).step_by(self.nb_instance)),
            )
        } else {
            None
        }
    }
}

pub struct QueryV2 {
    hquery: isize,
    _tx: mpsc::Sender<()>,
    save_path: path::PathBuf,
    hfile: Option<fs::File>,
    is_logging: bool,
    pub counters: collections::HashMap<usize, CounterV2>,
    last_id: usize,
    pub last_update: time::Instant,
}

impl QueryV2 {
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new(hwnd: HWND, menu: &mut menu::Menu) -> Self {
        let mut hquery = 0;
        assert!(PdhOpenQueryW(ptr::null(), 0, &mut hquery) == ERROR_SUCCESS);

        let (_tx, rx) = mpsc::channel::<()>();
        thread::spawn(move || loop {
            if let Err(TryRecvError::Disconnected) = rx.try_recv() {
                break;
            }

            SendMessageW(hwnd, WM_UPDATE_QUERY, 0, 0);
            thread::sleep(Duration::from_millis(990));
        });

        println!("{}", env::var("APP_DATA").expect("No APP_DATA directory"));

        let mut query_v2 = Self {
            hquery,
            _tx,
            save_path: env::current_dir().unwrap().join("save.json"),
            hfile: None,
            is_logging: false,
            counters: collections::HashMap::new(),
            last_id: 0,
            last_update: time::Instant::now(),
        };

        let saved_paths: Vec<Vec<u16>> = fs::read_to_string(&query_v2.save_path)
            .ok()
            .and_then(|string| serde_json::from_str(&string).ok())
            .unwrap_or_default();

        for path in saved_paths {
            query_v2.add_counter(hwnd, menu, Some(path));
        }

        query_v2.update(menu);
        query_v2
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn close(&self) {
        assert!(PdhCloseQuery(self.hquery) == ERROR_SUCCESS);

        if let Ok(data) = serde_json::to_string(
            &self
                .counters
                .values()
                .map(|counter| &counter.path)
                .collect::<Vec<_>>(),
        ) {
            fs::write(&self.save_path, data)
                .unwrap_or_else(|err| eprintln!("Unable to save counters path err({})", err));
        };
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn add_counter(
        &mut self,
        hwnd: HWND,
        menu: &mut menu::Menu,
        path: Option<Vec<u16>>,
    ) {
        if let Some(counter_v2) = CounterV2::new(hwnd, self.hquery, path) {
            menu.add_item(
                Some(menu::IDM_COUNTER_REMOVE),
                self.last_id as isize + 1 + menu::IDM_REMOVE_RANGE.start,
                counter_v2.path.as_ptr(),
                None,
                false,
            );

            self.counters.insert(self.last_id + 1, counter_v2);
            self.last_id += 1;
        };
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn remove_counter(&mut self, id: isize, menu: &mut menu::Menu) {
        menu.remove_item(
            Some(menu::IDM_COUNTER_REMOVE),
            id + menu::IDM_REMOVE_RANGE.start,
        );

        self.counters.remove(&(id as usize)).unwrap();
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn remove_all_counter(&mut self, menu: &mut menu::Menu) {
        for (id, _) in self.counters.drain() {
            menu.remove_item(
                Some(menu::IDM_COUNTER_REMOVE),
                id as isize + menu::IDM_REMOVE_RANGE.start,
            );
        }
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn update(&mut self, menu: &mut menu::Menu) {
        if PdhCollectQueryData(self.hquery) == ERROR_SUCCESS {
            let datas = self.counters.values_mut().map(|counter| counter.update());

            if self.is_logging {
                let mut sys_t: SYSTEMTIME = mem::zeroed();
                GetLocalTime(&mut sys_t);

                let tmp = datas
                    .map(|(counter_path, instance)| {
                        String::from_utf16(counter_path.as_slice()).unwrap()
                            + &(if let Some(instance_data) = instance {
                                instance_data
                                    .map(|(val, name)| {
                                        " ; (".to_string() + name + ", " + &val.to_string() + ")"
                                    })
                                    .collect::<String>()
                                    + " ; "
                            } else {
                                " ; (no data) ; ".to_string()
                            })
                    })
                    .collect::<String>();

                if writeln!(
                    self.hfile.as_ref().unwrap(),
                    "D{}-{}-{} T{}:{}:{}.{} ; {}",
                    sys_t.wYear,
                    sys_t.wMonth,
                    sys_t.wDay,
                    sys_t.wHour,
                    sys_t.wMinute,
                    sys_t.wSecond,
                    sys_t.wMilliseconds,
                    tmp
                )
                .is_err()
                {
                    self.stop_logging(menu);
                }
            } else {
                datas.for_each(drop);
            }

            self.last_update = time::Instant::now();
        }
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn start_logging(&mut self, menu: &mut menu::Menu, hwnd: HWND) {
        let mut sys_t: SYSTEMTIME = mem::zeroed();
        GetLocalTime(&mut sys_t);

        let mut file_name = format!("log_{}_{}_{}_{}", sys_t.wDay, sys_t.wMonth, sys_t.wHour, sys_t.wMinute)
            .encode_utf16()
            .chain(iter::repeat(0_u16).take(256))
            .collect::<Vec<_>>();

        let mut op = mem::zeroed::<OPENFILENAMEW>();
        op.lStructSize = mem::size_of::<OPENFILENAMEW>() as _;
        op.hwndOwner = hwnd;
        op.lpstrFile = file_name.as_mut_ptr();
        op.nMaxFile = file_name.len() as _;
        op.Flags = 0x00000400 | 0x00000800;
        op.lpstrDefExt = w!("pdhl");

        if GetSaveFileNameW(&mut op) != 1 {
            return;
        }

        let file_name_string = String::from_utf16(file_name.as_slice()).unwrap();

        if let Ok(hfile) = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(file_name_string.trim_matches(char::from(0)))
        {
            self.hfile = Some(hfile);
        }

        if self
            .hfile
            .as_ref()
            .unwrap()
            .set_len(0)
            .and(writeln!(self.hfile.as_ref().unwrap(), "copyright pdhv.fr",))
            .is_err()
        {
            return;
        }

        menu.set_item_state_by_pos(None, 0, None, true);
        menu.set_item_state_by_id(Some(menu::IDM_LOG), menu::IDM_LOG_START, None, true);
        menu.set_item_state_by_id(Some(menu::IDM_LOG), menu::IDM_LOG_STOP, None, false);

        self.is_logging = true;
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn stop_logging(&mut self, menu: &mut menu::Menu) {
        menu.set_item_state_by_pos(None, 0, None, false);
        menu.set_item_state_by_id(Some(menu::IDM_LOG), menu::IDM_LOG_START, None, false);
        menu.set_item_state_by_id(Some(menu::IDM_LOG), menu::IDM_LOG_STOP, None, true);

        self.is_logging = false;
    }
}

pub struct InterpolatedCurve {
    pub n: usize,

    a: Vec<f32>,
    b: Vec<f32>,
    c: Vec<f32>,
    d: Vec<f32>,
}

impl InterpolatedCurve {
    fn new(data: impl iter::Iterator<Item = f32>) -> Self {
        let a = Vec::from_iter(data);
        let n = a.len() - 1;

        let mut l = vec![1.0_f32];
        let mut u = vec![0.0_f32];
        let mut z = vec![0.0_f32];

        for i in 1..n {
            l.push(4.0 - u[i - 1]);
            u.push(1.0 / l[i]);
            z.push(
                ((3.0 * (a[i + 1] - a[i]) - 3.0 * (a[i] - a[i - 1])) - z[i - 1])
                    / l[i],
            );
        }
        l.push(1.0);
        z.push(0.0);

        let mut c = Vec::from_iter(iter::repeat(0.0_f32).take(n + 1));
        let mut b = Vec::from_iter(iter::repeat(0.0_f32).take(n));
        let mut d = Vec::from_iter(iter::repeat(0.0_f32).take(n));

        for j in (0..n).rev() {
            c[j] = z[j] - u[j] * c[j + 1];
            b[j] = (a[j + 1] - a[j]) - (c[j + 1] + 2.0 * c[j]) / 3.0;
            d[j] = (c[j + 1] - c[j]) / 3.0;
        }

        Self { n, a, b, c, d }
    }

    pub fn interpolate(&self, x: f32, j: usize) -> f32 {
        self.a[j]
            + self.b[j] * (x - j as f32)
            + self.c[j] * (x - j as f32).powf(2.0)
            + self.d[j] * (x - j as f32).powf(3.0)
    }
}
