// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::field_reassign_with_default)]
// Pełny pedantic z uzasadnionymi wyłączeniami hałaśliwych linterów,
// które nie mają sensu w aplikacji GUI (nie w bibliotece publicznej).
#![warn(clippy::pedantic)]
#![allow(
    // Konwersje liczbowe w DSP/audio/GUI (f32<->f64<->usize) — wymuszenie
    // jawnego obsłużenia każdego casta pogorszyłoby czytelność bez zysku.
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    // Funkcje egui (App::update) i panele są z natury długie.
    clippy::too_many_lines,
    // Struktury konfiguracyjne z wieloma flagami bool.
    clippy::struct_excessive_bools,
    // Aplikacja GUI, nie publiczna biblioteka — dokumentowanie każdego
    // błędu/paniki w `fn` byłoby szumem.
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    // Krótkie nazwy zmiennych w kodzie matematycznym/UI.
    clippy::similar_names,
    clippy::many_single_char_names,
    // Organizacja modułów przez glob-importy wewnętrzne.
    clippy::wildcard_imports,
    // Kosmetyka doc-komentarzy (backticki).
    clippy::doc_markdown,
    // Porównania float w DSP (progi szumu itp.).
    clippy::float_cmp
)]

pub mod cat;
pub mod cloud;
pub mod cluster;
pub mod core;
pub mod digital;
pub mod dsp;
pub mod gui;
pub mod media;
pub mod network;
pub mod plugins;
pub mod api;
pub mod sync;

