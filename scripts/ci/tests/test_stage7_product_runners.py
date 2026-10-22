import copy
import importlib.util
import json
import os
from pathlib import Path
import platform
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('product_runner', Path(__file__).parents[1] / 'stage7_product_runner.py')
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class ProductRunnerTests(unittest.TestCase):
    def test_percentiles_use_process_medians_but_max_uses_raw_samples(self):
        runs = [[10] * 9 + [10000], [20] * 10]
        self.assertEqual(runner.aggregate(runs), {'p50': 10, 'p95': 20, 'max': 10000})

    def test_relative_limit_is_inclusive_and_all_statistics_count(self):
        baseline = dict(p50=100, p95=100, max=100)
        self.assertTrue(runner.comparison(dict(p50=105, p95=105, max=105), baseline, 1.05)['passed'])
        self.assertFalse(runner.comparison(dict(p50=100, p95=100, max=106), baseline, 1.05)['passed'])
        with self.assertRaises(ValueError):
            runner.comparison(baseline, dict(p50=0, p95=100, max=100), 1.05)

    def fixture(self):
        protocol = dict(stabilization_ms=5000, sample_interval_ms=100, samples_per_process=10)
        data = dict(schema='rterm.diagnostics/v2', failures=[],
                    run=dict(platform='macos', architecture={'arm64': 'aarch64', 'x86_64': 'x86_64', 'AMD64': 'x86_64'}.get(platform.machine(), platform.machine()), scenario='ssh1', app_path='/tmp/app'),
                    configuration=dict(stabilization_ms=5000, sample_interval_ms=100, sample_count=10,
                                       columns=80, rows=24, scale_factor_milli=1000),
                    readiness=dict(status='ready'), renderer={'final': 'gpu'},
                    process=dict(exit_code=0, exit_kind='requested'), connection=dict(final_state='connected'),
                    memory=dict(metric='macos_phys_footprint_bytes', unit='bytes', samples=[dict(sequence=i, elapsed_ms=5100+i*100, bytes=100) for i in range(10)]),
                    milestones=dict(scenario_ready_ms=100))
        return data, protocol

    def test_actual_native_samples_are_required(self):
        data, protocol = self.fixture()
        self.assertEqual(runner.validate_residence(data, 'ssh1', Path('/tmp/app'), protocol), [100]*10)
        mutations = [
            lambda d: d['memory'].update(metric='rss'),
            lambda d: d['renderer'].update(final='cpu'),
            lambda d: d['connection'].update(final_state='disconnected'),
            lambda d: d['memory']['samples'].pop(),
            lambda d: d['memory']['samples'][0].update(elapsed_ms=1),
            lambda d: d['memory']['samples'][1].update(elapsed_ms=5101),
            lambda d: d['configuration'].update(requested_gpu_backend='gl'),
            lambda d: d['run'].update(app_path='/tmp/other'),
            lambda d: d['process'].update(exit_kind='forced'),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                invalid = copy.deepcopy(data)
                mutate(invalid)
                with self.assertRaises(ValueError):
                    runner.validate_residence(invalid, 'ssh1', Path('/tmp/app'), protocol)

    def test_failure_retains_stdout_and_receipt(self):
        import sys
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'failure'
            with self.assertRaises(ValueError):
                runner.run_record([sys.executable, '-c', "print('evidence'); raise SystemExit(7)"], output, 5)
            self.assertIn('evidence', (output/'stdout.txt').read_text())
            self.assertEqual(json.loads((output/'command.json').read_text())['returncode'], 7)

    @unittest.skipIf(os.name == 'nt', 'macOS runner uses POSIX process groups')
    def test_timeout_retains_evidence(self):
        import sys
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'timeout'
            with self.assertRaises(ValueError):
                runner.run_record([sys.executable, '-c', 'import time; time.sleep(5)'], output, 0.1)
            self.assertTrue(json.loads((output/'command.json').read_text())['timed_out'])

    def test_package_rejects_wrong_source_and_path_escape(self):
        with tempfile.TemporaryDirectory() as tmp:
            payload = Path(tmp).resolve()
            manifest = dict(package=dict(source_commit='a'*40), artifact=dict(binary='../outside'))
            (payload/'manifest.json').write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, 'source mismatch'):
                runner.package_identity(payload, 'b'*40)
            with self.assertRaisesRegex(ValueError, 'path escape'):
                runner.package_identity(payload, 'a'*40)


if __name__ == '__main__':
    unittest.main()
