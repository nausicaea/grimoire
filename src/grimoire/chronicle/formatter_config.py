from dataclasses import dataclass, field

from mashumaro import DataClassDictMixin

_DEFAULT_FORMAT: str = "[%(asctime)s] [%(levelname)s] [%(name)s] %(message)s"


@dataclass
class FormatterConfig(DataClassDictMixin):
    """
    Holds configuration for :class:`logging.Formatter`
    """

    format: str = field(default=_DEFAULT_FORMAT)
