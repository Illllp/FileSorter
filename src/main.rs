use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use chrono::{DateTime, Datelike, Local, NaiveDate};
use regex::Regex;
use once_cell::sync::Lazy;

// Список "фильтров" для поиска даты в имени файла.
static DATE_REGEXES: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"((?:19|20)\d{2})[-_.](\d{2})[-_.](\d{2})").unwrap(),
                                                  Regex::new(r"((?:19|20)\d{2})(\d{2})(\d{2})").unwrap(),
    ]
});

fn main() {
   
    let source_dir = prompt_for_path("Введите путь к папке с файлами для сортировки:");
    if !source_dir.is_dir() {
        eprintln!("Ошибка: Указанный исходный путь не существует или не является папкой.");
        process::exit(1);
    }
    let dest_dir = prompt_for_path("Введите путь к папке, куда будут перемещены файлы:");
    if let Err(e) = fs::create_dir_all(&dest_dir) {
        eprintln!("Не удалось создать папку назначения: {}", e);
        process::exit(1);
    }
    let no_date_dir_name = "Без Даты";
    let no_date_dir = dest_dir.join(no_date_dir_name);
    if let Err(e) = fs::create_dir_all(&no_date_dir) {
        eprintln!("Не удалось создать папку '{}': {}", no_date_dir_name, e);
        process::exit(1);
    }
    println!("Начинаю сортировку...");
    process_directory(&source_dir, &dest_dir, &no_date_dir);
    println!("Сортировка успешно завершена!");
}

fn process_directory(current_dir: &Path, dest_dir: &Path, no_date_dir: &Path) {
   
    match fs::read_dir(current_dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        if path != *dest_dir && path != *no_date_dir {
                            println!("Сканирую папку: {}", path.display());
                            process_directory(&path, dest_dir, no_date_dir);
                        }
                    } else if path.is_file() {
                        process_file(&path, dest_dir, no_date_dir);
                    }
                }
            }
        }
        Err(e) => eprintln!("Ошибка чтения директории {}: {}", current_dir.display(), e),
    }
}

fn process_file(file_path: &Path, dest_dir: &Path, no_date_dir: &Path) {
    let date = get_date_from_filename(file_path)
    .or_else(|| get_date_from_filesystem(file_path));

    let final_dest_dir = match date {
        Some(d) => {
            let year = d.year().to_string();
            let month = format!("{:02} - {}", d.month(), get_month_name_ru(d.month()));
            let day = format!("{:02}", d.day());
            dest_dir.join(year).join(month).join(day)
        }
        None => {
            println!("Не удалось определить дату для файла: {}. Перемещение в 'Без Даты'.", file_path.display());
            no_date_dir.to_path_buf()
        }
    };

    if let Err(e) = fs::create_dir_all(&final_dest_dir) {
        eprintln!("Не удалось создать директорию {}: {}", final_dest_dir.display(), e);
        return;
    }

    if let Some(file_name) = file_path.file_name() {
        let dest_file_path = final_dest_dir.join(file_name);
        if file_path != dest_file_path {
            println!("Перемещение {} -> {}", file_path.display(), dest_file_path.display());

            // === ГЛАВНОЕ ИЗМЕНЕНИЕ ДЛЯ РАБОТЫ МЕЖДУ ДИСКАМИ ===
            // Сначала пытаемся быстро переименовать
            match fs::rename(file_path, &dest_file_path) {
                // Если возникла ошибка...
                Err(e) => {
                    // ...и эта ошибка именно "Invalid cross-device link"...
                    if e.kind() == std::io::ErrorKind::CrossesDevices {
                        // ...тогда используем медленный, но надежный метод "копировать-удалить".
                        println!("    Перемещение между разными дисками, использую копирование...");
                        match fs::copy(file_path, &dest_file_path) {
                            Ok(_) => {
                                // Если копирование успешно, удаляем исходный файл
                                if let Err(remove_err) = fs::remove_file(file_path) {
                                    eprintln!("    Ошибка: Не удалось удалить исходный файл {} после копирования: {}", file_path.display(), remove_err);
                                }
                            }
                            Err(copy_err) => {
                                eprintln!("    Ошибка: Не удалось скопировать файл {}: {}", file_path.display(), copy_err);
                            }
                        }
                    } else {
                        // Если ошибка была другой (например, нет прав доступа), просто выводим ее.
                        eprintln!("    Не удалось переместить файл (ошибка: {})", e);
                    }
                }
                // Если ошибки не было, все хорошо, ничего не делаем.
                Ok(_) => {}
            }
        }
    }
}


fn get_date_from_filename(path: &Path) -> Option<NaiveDate> {
   
    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
        for re in DATE_REGEXES.iter() {
            if let Some(caps) = re.captures(file_name) {
                if let (Some(y_str), Some(m_str), Some(d_str)) = (caps.get(1), caps.get(2), caps.get(3)) {
                    if let (Ok(y), Ok(m), Ok(d)) = (y_str.as_str().parse(), m_str.as_str().parse(), d_str.as_str().parse()) {
                        if let Some(date) = NaiveDate::from_ymd_opt(y, m, d) {
                            return Some(date);
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_date_from_filesystem(path: &Path) -> Option<NaiveDate> {
   
    if let Ok(metadata) = fs::metadata(path) {
        let file_time = metadata.created().or_else(|_| metadata.modified());
        if let Ok(system_time) = file_time {
            let datetime: DateTime<Local> = system_time.into();
            return Some(datetime.date_naive());
        }
    }
    None
}

fn prompt_for_path(prompt_text: &str) -> PathBuf {
   
    let mut input_path = String::new();
    print!("{} ", prompt_text);
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input_path).expect("Не удалось прочитать строку");
    PathBuf::from(input_path.trim())
}

fn get_month_name_ru(month: u32) -> &'static str {
   
    match month {
        1 => "января", 2 => "февраля", 3 => "марта", 4 => "апреля", 5 => "мая",
        6 => "июня", 7 => "июля", 8 => "августа", 9 => "сентября",
        10 => "октября", 11 => "ноября", 12 => "декабря",
        _ => "неизвестный месяц",
    }
}
