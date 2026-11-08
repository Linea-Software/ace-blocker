"""Static regression tests for the Linux release delivery of the policy guard."""
from pathlib import Path
import unittest


REPOSITORY = Path(__file__).resolve().parents[3]
INSTALLER = REPOSITORY / "installer/linux/install.sh"
RELEASE = REPOSITORY / ".github/workflows/release.yml"
CI = REPOSITORY / ".github/workflows/ci.yml"


class LinuxBrowserPolicyPackagingTests(unittest.TestCase):
    def test_install_copies_helper_but_never_enables_it(self):
        shell = INSTALLER.read_text(encoding="utf-8")
        self.assertIn(
            'install -o root -g root -m 0755 "$package_dir/browser-proxy-policy.py"',
            shell,
        )
        self.assertIn(
            'Missing or unsafe browser proxy policy manager', shell
        )
        self.assertNotIn("browser-proxy-policy.py --enable\n", shell)
        # Installer only displays the opt-in command to a human.
        self.assertIn(
            'echo "To opt in after reviewing its effects: sudo '
            '$install_dir/browser-proxy-policy.py --enable"', shell
        )

    def test_linux_release_archives_helper_and_validates_tests(self):
        release = RELEASE.read_text(encoding="utf-8")
        self.assertIn(
            'install -m 0755 installer/linux/browser-proxy-policy.py '
            '"release-input/$package/browser-proxy-policy.py"', release
        )
        self.assertIn("python3 -m unittest discover -s installer/linux/tests", release)

    def test_linux_ci_runs_hermetic_policy_tests(self):
        ci = CI.read_text(encoding="utf-8")
        self.assertIn("Linux managed browser policy regression tests", ci)
        self.assertIn("python3 -m unittest discover -s installer/linux/tests", ci)


if __name__ == "__main__":
    unittest.main()
