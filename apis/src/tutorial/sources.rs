const SOURCES: &[(&str, &str, usize)] = &[
    ("basics.surround", "nmgMVg7s1QP3", 28),
    ("basics.placing", "5wmND8hIUz-R", 4),
    ("basics.queen_rule", "uQTf52SQHnk6", 6),
    ("basics.one_hive", "6XLwzobZAJP-", 10),
    ("basics.gates_and_doors", "T4RQlKs1xeWq", 14),
    ("basics.freedom_to_move", "e6NffjbJIjsN", 16),
    ("bugs.queen", "GO5IALkZn-Gi", 4),
    ("bugs.ant", "V-lEsxKH1A18", 8),
    ("bugs.spider", "VxWyE32npXeu", 10),
    ("bugs.spider_dead_end", "48scd9p83TA4", 26),
    ("bugs.grasshopper", "l2ZK3IAf9VqB", 22),
    ("bugs.beetle", "yoRTI-jLU3G1", 10),
    ("bugs.beetle_gate", "OiLpk05USK1S", 40),
    ("bugs.mosquito", "LrCAafNSTE4c", 8),
    ("bugs.ladybug", "BWt62rQLnjGv", 14),
    ("bugs.pillbug", "UPEe5FZNpX4x", 12),
    ("strategy.race", "EZCTc6hFPdJf", 30),
    ("strategy.pin", "B8vsmN1ARHEj", 6),
    ("strategy.gate", "t5VnqkqjUWGz", 14),
    ("strategy.two_for_one", "Dis-E6_cr_Ax", 12),
    ("strategy.shutout", "vJuDkF0t_GJZ", 46),
    ("strategy.bugzwang", "yDUpjkK3CA4l", 24),
    ("strategy.control", "AYVHlPI7Cn-k", 8),
    ("tactics.kill_spots", "U4lMk1GSBkwY", 8),
    ("tactics.true_and_false_pins", "ao3ripxEqpRg", 6),
    ("tactics.double_pin", "4SOUDPB7vx43", 20),
    ("tactics.pin_replacement", "tX-qbzdOqdea", 16),
    ("tactics.qualifying", "fIrZn9A9YgXg", 12),
    ("tactics.beetle_on_queen", "gWDjBMnEm7NM", 14),
    ("tactics.direct_drops", "P_ODFX2KIGh3", 16),
    ("tactics.recovery", "GjJNDWqk2n6q", 14),
    ("tactics.beetle_on_pillbug", "mng15cQMwQM-", 16),
    ("tactics.flood_pillbug", "psdrDSZkWLC3", 28),
    ("tactics.remove_pillbug", "QAunwD5p4xYj", 8),
    ("tactics.proximity_pillbug", "FVgIM_6_ED96", 18),
    ("tactics.queen_choke", "KBuoyBHpbDtW", 8),
    ("tactics.cavern", "s0iy6HU_97HA", 28),
    ("tactics.anti_spawn", "P_ODFX2KIGh3", 12),
    ("online.notation", "qjafieXD9LRn", 8),
    ("online.time_controls", "BWt62rQLnjGv", 4),
];

pub fn source_game(lesson_id: &str) -> Option<String> {
    SOURCES
        .iter()
        .find(|(id, _, _)| *id == lesson_id)
        .map(|(_, game, moves)| format!("https://hivegame.com/game/{game}?move={moves}"))
}
