from grimoire.chronicle.formatter_config import (_DEFAULT_FORMAT,
                                                 FormatterConfig)


def test_formatter_config_has_format_field_with_default_value() -> None:
    assert FormatterConfig().format == _DEFAULT_FORMAT
