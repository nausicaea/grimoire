try:
    from dataclasses import dataclass, field
    from pathlib import Path

    from mashumaro import DataClassDictMixin, field_options

    @dataclass
    class HandlerConfig(DataClassDictMixin):
        """
        Holds configuration for :class:`logging.Handler`
        """

        class_: str = field(metadata=field_options(alias="class"))
        level: int | None = field(default=None)
        formatter: str | None = field(default=None)
        stream: str | None = field(default=None)
        filename: str | None = field(default=None)
        delay: bool | None = field(default=None)
        address: str | None = field(default=None)
        rich_tracebacks: bool | None = field(default=None)
        maxBytes: int = field(default=0)
        backupCount: int = field(default=0)

        @classmethod
        def stderr_stream_handler(cls, level: int, formatter: str) -> "HandlerConfig":
            """
            Create a configuration entry for :class:`logging.StreamHandler`

            :param level: a logging level
            :param formatter: the name of a formatter
            :return:
            """
            return HandlerConfig(
                class_="logging.StreamHandler",
                formatter=formatter,
                level=level,
                stream="ext://sys.stderr",
            )

        @classmethod
        def rich_handler(
            cls, level: int, rich_tracebacks: bool = True
        ) -> "HandlerConfig":
            """
            Create a configuration entry for :class:`rich.logging.RichHandler`

            :param level: a logging level
            :param formatter: the name of a formatter
            :return:
            """
            return HandlerConfig(
                class_="rich.logging.RichHandler",
                level=level,
                rich_tracebacks=rich_tracebacks,
            )

        @classmethod
        def file_handler(
            cls, file_name: Path, level: int, formatter: str
        ) -> "HandlerConfig":
            """
            Create a configuration entry for :class:`logging.FileHandler`

            :param file_name: the name of the log output file
            :param level: a logging level
            :param formatter: the name of a formatter
            :return:
            """
            return HandlerConfig(
                class_="logging.FileHandler",
                formatter=formatter,
                level=level,
                filename=str(file_name),
                delay=True,
            )

        @classmethod
        def rotating_file_handler(
            cls,
            file_name: Path,
            level: int,
            formatter: str,
            max_bytes: int = 0,
            backup_count: int = 0,
        ) -> "HandlerConfig":
            """
            Create a configuration entry for :class:`logging.handlers.RotatingFileHandler`

            :param file_name: the name of the log output file
            :param level: a logging level
            :param formatter: the name of a formatter
            :param max_bytes: the maximum size of a log file
            :param backup_count: the number of log file backups to keep
            :return:
            """
            return HandlerConfig(
                class_="logging.handlers.RotatingFileHandler",
                formatter=formatter,
                level=level,
                filename=str(file_name),
                delay=True,
                maxBytes=max_bytes,
                backupCount=backup_count,
            )

        @classmethod
        def journald_handler(cls, level: int, formatter: str) -> "HandlerConfig":
            """
            Create a configuration entry for :class:`systemd.journal.JournalHandler` from the package systemd-python

            :param level: a logging level
            :param formatter: the name of a formatter
            :return:
            """
            return HandlerConfig(
                class_="systemd.journal.JournalHandler",
                formatter=formatter,
                level=level,
            )

        @classmethod
        def syslog_handler(
            cls, level: int, formatter: str, address: str = "/dev/log"
        ) -> "HandlerConfig":
            """
            Create a configuration entry for :class:`logging.handlers.SysLogHandler`

            :param level: a logging level
            :param formatter: the name of a formatter
            :return:
            """
            return HandlerConfig(
                class_="logging.handlers.SysLogHandler",
                formatter=formatter,
                level=level,
                address=address,
            )

except ImportError as ex:
    mashumaro = ex
