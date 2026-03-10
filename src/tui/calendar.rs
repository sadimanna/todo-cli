use chrono::{Datelike, Duration, Local, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarTarget {
    Deadline,
    Reminder,
}

#[derive(Debug, Clone)]
pub struct CalendarState {
    pub selected_day: u32,
    pub selected_month: u32,
    pub selected_year: i32,
}

impl CalendarState {
    pub fn today() -> Self {
        let today = Local::now().date_naive();
        Self {
            selected_day: today.day(),
            selected_month: today.month(),
            selected_year: today.year(),
        }
    }

    pub fn selected_date(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.selected_year, self.selected_month, self.selected_day).unwrap()
    }

    pub fn move_days(&mut self, delta: i64) {
        let date = self.selected_date() + Duration::days(delta);
        self.set_date(date);
        self.clamp_to_today();
    }

    pub fn move_months(&mut self, delta: i32) {
        let mut year = self.selected_year;
        let mut month = self.selected_month as i32 + delta;
        while month > 12 {
            month -= 12;
            year += 1;
        }
        while month < 1 {
            month += 12;
            year -= 1;
        }
        let month_u32 = month as u32;
        let days = days_in_month(year, month_u32);
        let day = self.selected_day.min(days);
        let date = NaiveDate::from_ymd_opt(year, month_u32, day).unwrap();
        self.set_date(date);
        self.clamp_to_today();
    }

    fn set_date(&mut self, date: NaiveDate) {
        self.selected_day = date.day();
        self.selected_month = date.month();
        self.selected_year = date.year();
    }

    fn clamp_to_today(&mut self) {
        let today = Local::now().date_naive();
        if self.selected_date() < today {
            self.set_date(today);
        }
    }
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    (next - Duration::days(1)).day()
}

pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    }
}
