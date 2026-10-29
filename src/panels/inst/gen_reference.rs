pub const REFERENCE: Reference = Reference {
    size: [14.0, 13.99],
    settings: &[],
    cases: &[
        Case {
            name: "Legacy 0",
            tab: 0,
            params: &[],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(2, Widget::Spinner, "Algorithm", [0.00, 1.60, 3.50, 3.20]).p(1),
                row(3, Widget::Knob, "Feedback", [3.50, 1.60, 3.50, 3.20]).p(2),
                row(4, Widget::Knob, "FM Level", [7.00, 1.60, 3.50, 3.20]).p(4),
                row(5, Widget::Knob, "Master", [10.50, 1.60, 3.50, 3.20]).p(3),
                row(6, Widget::Knob, "Drive", [0.00, 4.80, 3.50, 3.20]).p(9),
                row(7, Widget::Knob, "DAC Color", [3.50, 4.80, 3.50, 3.20]).p(10),
                row(8, Widget::Knob, "Stereo Width", [7.00, 4.80, 3.50, 3.20]).p(11),
                row(9, Widget::Spinner, "Engine", [10.50, 4.80, 3.50, 3.20]).p(54),
                row(59, Widget::Spinner, "FM Model", [0.00, 8.00, 3.50, 2.40]).p(56),
                row(60, Widget::HSlider, "Chip Region", [3.50, 8.00, 3.50, 2.40])
                    .p(57)
                    .enabled(Show::Not(&Show::All(&[
                        Show::Item(MODEL, 0),
                        Show::Item(58, 0),
                    ]))),
                row(61, Widget::Spinner, "PSG Model", [7.00, 8.00, 3.50, 2.40]).p(58),
                row(
                    62,
                    Widget::HSlider,
                    "Chip Velocity",
                    [10.50, 8.00, 3.50, 2.40],
                )
                .p(61)
                .enabled(Show::Item(MODEL, 1)),
                row(65, Widget::Display, "", [0.00, 10.40, 14.00, 2.30]).h(Size::U(2.30)),
                row(58, Widget::Display, "", [0.00, 12.70, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "Legacy 1",
            tab: 1,
            params: &[],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(10, Widget::Spinner, "Ratio", [0.00, 1.60, 3.50, 3.20]).p(12),
                row(11, Widget::Knob, "Detune", [3.50, 1.60, 3.50, 3.20]).p(13),
                row(12, Widget::Knob, "Level", [7.00, 1.60, 3.50, 3.20]).p(14),
                row(13, Widget::Knob, "Attack", [10.50, 1.60, 3.50, 3.20]).p(15),
                row(14, Widget::Knob, "Decay", [0.00, 4.80, 3.50, 3.20]).p(16),
                row(15, Widget::Knob, "Sustain", [3.50, 4.80, 3.50, 3.20]).p(17),
                row(16, Widget::Knob, "Release", [7.00, 4.80, 3.50, 3.20]).p(18),
                row(17, Widget::Knob, "Amp Mod", [10.50, 4.80, 3.50, 3.20]).p(19),
                row(66, Widget::Display, "", [0.00, 8.00, 14.00, 2.60]).h(Size::U(2.60)),
                row(58, Widget::Display, "", [0.00, 10.60, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "Legacy 2",
            tab: 2,
            params: &[],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(18, Widget::Spinner, "Ratio", [0.00, 1.60, 3.50, 3.20]).p(20),
                row(19, Widget::Knob, "Detune", [3.50, 1.60, 3.50, 3.20]).p(21),
                row(20, Widget::Knob, "Level", [7.00, 1.60, 3.50, 3.20]).p(22),
                row(21, Widget::Knob, "Attack", [10.50, 1.60, 3.50, 3.20]).p(23),
                row(22, Widget::Knob, "Decay", [0.00, 4.80, 3.50, 3.20]).p(24),
                row(23, Widget::Knob, "Sustain", [3.50, 4.80, 3.50, 3.20]).p(25),
                row(24, Widget::Knob, "Release", [7.00, 4.80, 3.50, 3.20]).p(26),
                row(25, Widget::Knob, "Amp Mod", [10.50, 4.80, 3.50, 3.20]).p(27),
                row(66, Widget::Display, "", [0.00, 8.00, 14.00, 2.60]).h(Size::U(2.60)),
                row(58, Widget::Display, "", [0.00, 10.60, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "Legacy 3",
            tab: 3,
            params: &[],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(26, Widget::Spinner, "Ratio", [0.00, 1.60, 3.50, 3.20]).p(28),
                row(27, Widget::Knob, "Detune", [3.50, 1.60, 3.50, 3.20]).p(29),
                row(28, Widget::Knob, "Level", [7.00, 1.60, 3.50, 3.20]).p(30),
                row(29, Widget::Knob, "Attack", [10.50, 1.60, 3.50, 3.20]).p(31),
                row(30, Widget::Knob, "Decay", [0.00, 4.80, 3.50, 3.20]).p(32),
                row(31, Widget::Knob, "Sustain", [3.50, 4.80, 3.50, 3.20]).p(33),
                row(32, Widget::Knob, "Release", [7.00, 4.80, 3.50, 3.20]).p(34),
                row(33, Widget::Knob, "Amp Mod", [10.50, 4.80, 3.50, 3.20]).p(35),
                row(66, Widget::Display, "", [0.00, 8.00, 14.00, 2.60]).h(Size::U(2.60)),
                row(58, Widget::Display, "", [0.00, 10.60, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "Legacy 4",
            tab: 4,
            params: &[],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(34, Widget::Spinner, "Ratio", [0.00, 1.60, 3.50, 3.20]).p(36),
                row(35, Widget::Knob, "Detune", [3.50, 1.60, 3.50, 3.20]).p(37),
                row(36, Widget::Knob, "Level", [7.00, 1.60, 3.50, 3.20]).p(38),
                row(37, Widget::Knob, "Attack", [10.50, 1.60, 3.50, 3.20]).p(39),
                row(38, Widget::Knob, "Decay", [0.00, 4.80, 3.50, 3.20]).p(40),
                row(39, Widget::Knob, "Sustain", [3.50, 4.80, 3.50, 3.20]).p(41),
                row(40, Widget::Knob, "Release", [7.00, 4.80, 3.50, 3.20]).p(42),
                row(41, Widget::Knob, "Amp Mod", [10.50, 4.80, 3.50, 3.20]).p(43),
                row(66, Widget::Display, "", [0.00, 8.00, 14.00, 2.60]).h(Size::U(2.60)),
                row(58, Widget::Display, "", [0.00, 10.60, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "Legacy 5",
            tab: 5,
            params: &[],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(42, Widget::Knob, "LFO Rate", [0.00, 1.60, 3.50, 3.20]).p(6),
                row(43, Widget::Knob, "Pitch LFO", [3.50, 1.60, 3.50, 3.20]).p(7),
                row(44, Widget::Knob, "Amp LFO", [7.00, 1.60, 3.50, 3.20]).p(8),
                row(45, Widget::Knob, "Portamento", [10.50, 1.60, 3.50, 3.20]).p(52),
                row(68, Widget::Display, "", [0.00, 4.80, 14.00, 2.40]).h(Size::U(2.40)),
                row(58, Widget::Display, "", [0.00, 7.20, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "Legacy 6",
            tab: 6,
            params: &[],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(46, Widget::Knob, "PSG A", [0.00, 1.60, 3.50, 3.20]).p(44),
                row(47, Widget::Knob, "PSG B", [3.50, 1.60, 3.50, 3.20]).p(45),
                row(48, Widget::Knob, "PSG C", [7.00, 1.60, 3.50, 3.20]).p(46),
                row(49, Widget::Knob, "Noise", [10.50, 1.60, 3.50, 3.20]).p(47),
                row(50, Widget::Spinner, "Noise Type", [0.00, 4.80, 3.50, 3.20]).p(48),
                row(51, Widget::Knob, "Noise Rate", [3.50, 4.80, 3.50, 3.20]).p(49),
                row(52, Widget::Knob, "PSG Spread", [7.00, 4.80, 3.50, 3.20]).p(50),
                row(53, Widget::Knob, "PSG Mix", [10.50, 4.80, 3.50, 3.20]).p(5),
                row(58, Widget::Display, "", [0.00, 8.00, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "Legacy 7",
            tab: 7,
            params: &[],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(54, Widget::Spinner, "Engine", [0.00, 1.60, 3.50, 3.20]).p(54),
                row(55, Widget::Knob, "Output Trim", [3.50, 1.60, 3.50, 3.20]).p(55),
                row(56, Widget::Knob, "Pitch Bend", [7.00, 1.60, 3.50, 3.20]).p(51),
                row(57, Widget::Knob, "Release Tail", [10.50, 1.60, 3.50, 3.20]).p(53),
                row(77, Widget::Knob, "Master", [0.00, 4.80, 3.50, 3.20]).p(3),
                row(78, Widget::Knob, "FM Level", [3.50, 4.80, 3.50, 3.20]).p(4),
                row(79, Widget::Knob, "PSG Level", [7.00, 4.80, 3.50, 3.20]).p(5),
                row(80, Widget::Knob, "Stereo Width", [10.50, 4.80, 3.50, 3.20]).p(11),
                row(67, Widget::Display, "", [0.00, 8.00, 14.00, 1.40]).h(Size::U(1.40)),
                row(58, Widget::Display, "", [0.00, 9.40, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "YM2612 0",
            tab: 0,
            params: &[(56, 1.0), (58, 1.0)],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(2, Widget::Spinner, "Algorithm", [0.00, 1.60, 3.50, 3.20]).p(1),
                row(3, Widget::Knob, "Feedback", [3.50, 1.60, 3.50, 3.20]).p(2),
                row(4, Widget::Knob, "FM Level", [7.00, 1.60, 3.50, 3.20]).p(4),
                row(5, Widget::Knob, "Master", [10.50, 1.60, 3.50, 3.20]).p(3),
                row(6, Widget::Knob, "Drive", [0.00, 4.80, 3.50, 3.20]).p(9),
                row(7, Widget::Knob, "DAC Color", [3.50, 4.80, 3.50, 3.20]).p(10),
                row(8, Widget::Knob, "Stereo Width", [7.00, 4.80, 3.50, 3.20]).p(11),
                row(9, Widget::Spinner, "Engine", [10.50, 4.80, 3.50, 3.20]).p(54),
                row(59, Widget::Spinner, "FM Model", [0.00, 8.00, 3.50, 2.40]).p(56),
                row(60, Widget::HSlider, "Chip Region", [3.50, 8.00, 3.50, 2.40])
                    .p(57)
                    .enabled(Show::Not(&Show::All(&[
                        Show::Item(MODEL, 0),
                        Show::Item(58, 0),
                    ]))),
                row(61, Widget::Spinner, "PSG Model", [7.00, 8.00, 3.50, 2.40]).p(58),
                row(
                    62,
                    Widget::HSlider,
                    "Chip Velocity",
                    [10.50, 8.00, 3.50, 2.40],
                )
                .p(61)
                .enabled(Show::Item(MODEL, 1)),
                row(65, Widget::Display, "", [0.00, 10.40, 14.00, 2.30]).h(Size::U(2.30)),
                row(58, Widget::Display, "", [0.00, 12.70, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "YM2612 1",
            tab: 1,
            params: &[(56, 1.0), (58, 1.0)],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(100, Widget::Knob, "Attack rate", [0.00, 1.60, 3.50, 2.80]).p(62),
                row(101, Widget::Knob, "Decay rate", [3.50, 1.60, 3.50, 2.80]).p(63),
                row(102, Widget::Knob, "Sustain rate", [7.00, 1.60, 3.50, 2.80]).p(64),
                row(
                    103,
                    Widget::Spinner,
                    "Sustain level",
                    [10.50, 1.60, 3.50, 2.80],
                )
                .p(65),
                row(104, Widget::Knob, "Release rate", [0.00, 4.40, 3.50, 2.80]).p(66),
                row(105, Widget::Knob, "Rate scale", [3.50, 4.40, 3.50, 2.80]).p(67),
                row(106, Widget::Knob, "Total level", [7.00, 4.40, 3.50, 2.80]).p(68),
                row(107, Widget::Spinner, "Detune", [10.50, 4.40, 3.50, 2.80]).p(69),
                row(108, Widget::Spinner, "SSG-EG", [0.00, 7.20, 3.50, 2.80]).p(70),
                row(109, Widget::Spinner, "Multiple", [3.50, 7.20, 3.50, 2.80]).p(71),
                row(110, Widget::Toggle, "AM enable", [7.00, 7.20, 3.50, 2.80]).p(102),
                row(111, Widget::Knob, "Master", [10.50, 7.20, 3.50, 2.80]).p(3),
                row(66, Widget::Display, "", [0.00, 10.00, 14.00, 2.60]).h(Size::U(2.60)),
                row(58, Widget::Display, "", [0.00, 12.60, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "YM2612 2",
            tab: 2,
            params: &[(56, 1.0), (58, 1.0)],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(116, Widget::Knob, "Attack rate", [0.00, 1.60, 3.50, 2.80]).p(72),
                row(117, Widget::Knob, "Decay rate", [3.50, 1.60, 3.50, 2.80]).p(73),
                row(118, Widget::Knob, "Sustain rate", [7.00, 1.60, 3.50, 2.80]).p(74),
                row(
                    119,
                    Widget::Spinner,
                    "Sustain level",
                    [10.50, 1.60, 3.50, 2.80],
                )
                .p(75),
                row(120, Widget::Knob, "Release rate", [0.00, 4.40, 3.50, 2.80]).p(76),
                row(121, Widget::Knob, "Rate scale", [3.50, 4.40, 3.50, 2.80]).p(77),
                row(122, Widget::Knob, "Total level", [7.00, 4.40, 3.50, 2.80]).p(78),
                row(123, Widget::Spinner, "Detune", [10.50, 4.40, 3.50, 2.80]).p(79),
                row(124, Widget::Spinner, "SSG-EG", [0.00, 7.20, 3.50, 2.80]).p(80),
                row(125, Widget::Spinner, "Multiple", [3.50, 7.20, 3.50, 2.80]).p(81),
                row(126, Widget::Toggle, "AM enable", [7.00, 7.20, 3.50, 2.80]).p(103),
                row(127, Widget::Knob, "Master", [10.50, 7.20, 3.50, 2.80]).p(3),
                row(66, Widget::Display, "", [0.00, 10.00, 14.00, 2.60]).h(Size::U(2.60)),
                row(58, Widget::Display, "", [0.00, 12.60, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "YM2612 3",
            tab: 3,
            params: &[(56, 1.0), (58, 1.0)],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(132, Widget::Knob, "Attack rate", [0.00, 1.60, 3.50, 2.80]).p(82),
                row(133, Widget::Knob, "Decay rate", [3.50, 1.60, 3.50, 2.80]).p(83),
                row(134, Widget::Knob, "Sustain rate", [7.00, 1.60, 3.50, 2.80]).p(84),
                row(
                    135,
                    Widget::Spinner,
                    "Sustain level",
                    [10.50, 1.60, 3.50, 2.80],
                )
                .p(85),
                row(136, Widget::Knob, "Release rate", [0.00, 4.40, 3.50, 2.80]).p(86),
                row(137, Widget::Knob, "Rate scale", [3.50, 4.40, 3.50, 2.80]).p(87),
                row(138, Widget::Knob, "Total level", [7.00, 4.40, 3.50, 2.80]).p(88),
                row(139, Widget::Spinner, "Detune", [10.50, 4.40, 3.50, 2.80]).p(89),
                row(140, Widget::Spinner, "SSG-EG", [0.00, 7.20, 3.50, 2.80]).p(90),
                row(141, Widget::Spinner, "Multiple", [3.50, 7.20, 3.50, 2.80]).p(91),
                row(142, Widget::Toggle, "AM enable", [7.00, 7.20, 3.50, 2.80]).p(104),
                row(143, Widget::Knob, "Master", [10.50, 7.20, 3.50, 2.80]).p(3),
                row(66, Widget::Display, "", [0.00, 10.00, 14.00, 2.60]).h(Size::U(2.60)),
                row(58, Widget::Display, "", [0.00, 12.60, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "YM2612 4",
            tab: 4,
            params: &[(56, 1.0), (58, 1.0)],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(148, Widget::Knob, "Attack rate", [0.00, 1.60, 3.50, 2.80]).p(92),
                row(149, Widget::Knob, "Decay rate", [3.50, 1.60, 3.50, 2.80]).p(93),
                row(150, Widget::Knob, "Sustain rate", [7.00, 1.60, 3.50, 2.80]).p(94),
                row(
                    151,
                    Widget::Spinner,
                    "Sustain level",
                    [10.50, 1.60, 3.50, 2.80],
                )
                .p(95),
                row(152, Widget::Knob, "Release rate", [0.00, 4.40, 3.50, 2.80]).p(96),
                row(153, Widget::Knob, "Rate scale", [3.50, 4.40, 3.50, 2.80]).p(97),
                row(154, Widget::Knob, "Total level", [7.00, 4.40, 3.50, 2.80]).p(98),
                row(155, Widget::Spinner, "Detune", [10.50, 4.40, 3.50, 2.80]).p(99),
                row(156, Widget::Spinner, "SSG-EG", [0.00, 7.20, 3.50, 2.80]).p(100),
                row(157, Widget::Spinner, "Multiple", [3.50, 7.20, 3.50, 2.80]).p(101),
                row(158, Widget::Toggle, "AM enable", [7.00, 7.20, 3.50, 2.80]).p(105),
                row(159, Widget::Knob, "Master", [10.50, 7.20, 3.50, 2.80]).p(3),
                row(66, Widget::Display, "", [0.00, 10.00, 14.00, 2.60]).h(Size::U(2.60)),
                row(58, Widget::Display, "", [0.00, 12.60, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "YM2612 5",
            tab: 5,
            params: &[(56, 1.0), (58, 1.0)],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(63, Widget::Spinner, "Chip LFO", [0.00, 1.60, 3.50, 3.20]).p(60),
                row(43, Widget::Knob, "Pitch LFO", [3.50, 1.60, 3.50, 3.20]).p(7),
                row(44, Widget::Knob, "Amp LFO", [7.00, 1.60, 3.50, 3.20]).p(8),
                row(45, Widget::Knob, "Portamento", [10.50, 1.60, 3.50, 3.20]).p(52),
                row(
                    76,
                    Widget::HSlider,
                    "Follow rate (Hz)",
                    [0.00, 4.80, 14.00, 1.20],
                )
                .p(6)
                .h(Size::U(1.20))
                .enabled(Show::Item(60, 0)),
                row(68, Widget::Display, "", [0.00, 6.00, 14.00, 2.40]).h(Size::U(2.40)),
                row(58, Widget::Display, "", [0.00, 8.40, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "YM2612 6",
            tab: 6,
            params: &[(56, 1.0), (58, 1.0)],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(46, Widget::Knob, "PSG A", [0.00, 1.60, 3.50, 3.20]).p(44),
                row(47, Widget::Knob, "PSG B", [3.50, 1.60, 3.50, 3.20]).p(45),
                row(48, Widget::Knob, "PSG C", [7.00, 1.60, 3.50, 3.20]).p(46),
                row(49, Widget::Knob, "Noise", [10.50, 1.60, 3.50, 3.20]).p(47),
                row(50, Widget::Spinner, "Noise Type", [0.00, 4.80, 3.50, 3.20]).p(48),
                row(64, Widget::Spinner, "Noise Clock", [3.50, 4.80, 3.50, 3.20]).p(59),
                row(52, Widget::Knob, "PSG Spread", [7.00, 4.80, 3.50, 3.20]).p(50),
                row(53, Widget::Knob, "PSG Mix", [10.50, 4.80, 3.50, 3.20]).p(5),
                row(58, Widget::Display, "", [0.00, 8.00, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "YM2612 7",
            tab: 7,
            params: &[(56, 1.0), (58, 1.0)],
            rows: &[
                row(
                    1,
                    Widget::Tabs,
                    "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT",
                    [0.00, 0.00, 14.00, 1.60],
                )
                .h(Size::U(1.60)),
                row(54, Widget::Spinner, "Engine", [0.00, 1.60, 3.50, 3.20]).p(54),
                row(55, Widget::Knob, "Output Trim", [3.50, 1.60, 3.50, 3.20]).p(55),
                row(56, Widget::Knob, "Pitch Bend", [7.00, 1.60, 3.50, 3.20]).p(51),
                row(57, Widget::Knob, "Release Tail", [10.50, 1.60, 3.50, 3.20]).p(53),
                row(77, Widget::Knob, "Master", [0.00, 4.80, 3.50, 3.20]).p(3),
                row(78, Widget::Knob, "FM Level", [3.50, 4.80, 3.50, 3.20]).p(4),
                row(79, Widget::Knob, "PSG Level", [7.00, 4.80, 3.50, 3.20]).p(5),
                row(80, Widget::Knob, "Stereo Width", [10.50, 4.80, 3.50, 3.20]).p(11),
                row(67, Widget::Display, "", [0.00, 8.00, 14.00, 1.40]).h(Size::U(1.40)),
                row(58, Widget::Display, "", [0.00, 9.40, 14.00, 1.20]).h(Size::U(1.20)),
            ],
            ..Case::DEFAULT
        },
    ],
};
