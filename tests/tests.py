from unittest import TestCase

import maturin_import_hook

from .snapshot import compare_snapshot

# install the import hook with default settings.
# this call must be before any imports that you want the hook to be active for.
maturin_import_hook.install()

# when a rust package that is installed in editable mode is imported,
# that package will be automatically recompiled if necessary.

import fast_pdf_extract  # noqa


class TestCases(TestCase):
    def test_pdf(self):
        pages = fast_pdf_extract.get_pages("tests/test_files/mini-pnb.pdf")
        compare_snapshot("\n\n".join(pages), "tests/test_files/mini-pnb.txt")

    def test_header_footer(self):
        pages = fast_pdf_extract.get_pages("tests/test_files/concall.pdf")
        compare_snapshot("\n\n".join(pages), "tests/test_files/concall.txt")

    def test_bad_file(self):
        with self.assertRaises(IOError):
            fast_pdf_extract.get_pages("tests/test_files/foo.txt")

    def test_null_char(self):
        pages = fast_pdf_extract.get_pages("tests/test_files/debug-ar.pdf")
        compare_snapshot("\n\n".join(pages), "tests/test_files/debug-ar.txt")

    def test_i32_box(self):
        pages = fast_pdf_extract.get_pages("tests/test_files/i32-box.pdf")
        compare_snapshot("\n\n".join(pages), "tests/test_files/i32-box.txt")

    def test_bad_json(self):
        pages = fast_pdf_extract.get_pages("tests/test_files/bad-json.pdf")
        compare_snapshot("\n\n".join(pages), "tests/test_files/bad-json.txt")
