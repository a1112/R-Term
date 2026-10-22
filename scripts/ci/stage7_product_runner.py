"""macOS product sampling; local evidence, never a cross-platform GO certificate."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')
    temporary.replace(path)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def rank(values, percentile):
    require(bool(values), 'no samples')
    return sorted(values)[math.ceil(len(values) * percentile / 100) - 1]


def aggregate(runs):
    representatives = [rank(values, 50) for values in runs]
    return {'p50': rank(representatives, 50), 'p95': rank(representatives, 95),
            'max': max(max(values) for values in runs)}


def comparison(candidate, baseline, limit):
    require(all(baseline[k] > 0 for k in ('p50', 'p95', 'max')), 'zero baseline')
    ratios = {k: candidate[k] / baseline[k] for k in ('p50', 'p95', 'max')}
    return {'ratios': ratios, 'limit': limit, 'passed': all(v <= limit for v in ratios.values())}


def validate_residence(data, scenario, binary, protocol):
    require(data['schema'] == 'rterm.diagnostics/v2', 'wrong schema')
    require(data['failures'] == [], 'launcher reported failures')
    run = data['run']
    require(run['platform'] == 'macos', 'wrong platform')
    require(run['architecture'] == {'arm64': 'aarch64', 'aarch64': 'aarch64', 'AMD64': 'x86_64', 'x86_64': 'x86_64'}[platform.machine()], 'wrong architecture')
    require(run['scenario'] == scenario.replace('-', '_'), 'wrong scenario')
    require(Path(run['app_path']).resolve() == binary.resolve(), 'wrong executable')
    cfg = data['configuration']
    require((cfg['columns'], cfg['rows'], cfg['scale_factor_milli']) == (80, 24, 1000), 'window geometry changed')
    for key in ('stabilization_ms', 'sample_interval_ms'):
        require(cfg[key] == protocol[key], 'wrong sampling protocol: ' + key)
    require(cfg['sample_count'] == protocol['samples_per_process'], 'wrong sample count')
    require(cfg.get('requested_renderer', 'auto') == 'auto', 'renderer override')
    require(not any(cfg.get(k) is not None for k in ('requested_gpu_backend', 'requested_font_mode', 'requested_font_specimen', 'requested_attribution_stage')), 'diagnostic override')
    require(data['readiness']['status'] == 'ready', 'not ready')
    require(data['renderer']['final'] == 'gpu', 'GPU product path required')
    require(data['process']['exit_code'] == 0 and data['process']['exit_kind'] != 'forced', 'unclean exit')
    if scenario == 'ssh1':
        require(data['connection']['final_state'] == 'connected', 'SSH not connected')
    else:
        require(data['connection']['final_state'] == 'not_started', 'empty window started transport')
    memory = data['memory']
    require(memory['metric'] == 'macos_phys_footprint_bytes' and memory['unit'] == 'bytes', 'native physical footprint required')
    samples = memory['samples']
    require(len(samples) == protocol['samples_per_process'], 'missing samples')
    require([s['sequence'] for s in samples] == list(range(len(samples))), 'sample sequence mismatch')
    require(all(type(s['bytes']) is int and s['bytes'] > 0 for s in samples), 'invalid byte sample')
    ready = data['milestones']['scenario_ready_ms']
    require(ready is not None, 'missing owner-ready milestone')
    require(samples[0]['elapsed_ms'] >= ready + protocol['stabilization_ms'], 'sample before stabilization')
    require(all(b['elapsed_ms'] - a['elapsed_ms'] >= protocol['sample_interval_ms'] for a, b in zip(samples, samples[1:])), 'samples too close')
    return [s['bytes'] for s in samples]


def run_record(argv, directory, timeout, env=None):
    directory.mkdir(parents=True, exist_ok=False)
    started = time.monotonic()
    with (directory / 'stdout.txt').open('wb') as out, (directory / 'stderr.txt').open('wb') as err:
        child = subprocess.Popen(argv, stdout=out, stderr=err, start_new_session=True, env=env)
        timed_out = False
        try:
            code = child.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(child.pid, signal.SIGKILL)
            code = child.wait()
        except BaseException:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()
            raise
    record = {'argv': argv, 'returncode': code, 'timed_out': timed_out,
              'elapsed_seconds': time.monotonic() - started,
              'stdout_sha256': digest(directory / 'stdout.txt'),
              'stderr_sha256': digest(directory / 'stderr.txt')}
    write_json(directory / 'command.json', record)
    require(code == 0 and not timed_out, f'command failed; evidence: {directory}')
    return (directory / 'stdout.txt').read_text(), (directory / 'stderr.txt').read_text()


def package_identity(payload, commit):
    require(payload.is_dir() and not payload.is_symlink(), 'invalid package directory')
    manifest_path = payload / 'manifest.json'
    manifest = json.loads(manifest_path.read_text())
    require(manifest['package']['source_commit'] == commit, 'package source mismatch')
    relative = Path(manifest['artifact']['binary'])
    require(not relative.is_absolute() and '..' not in relative.parts, 'package path escape')
    binary = payload / relative
    for part in (binary, *binary.parents):
        require(not part.is_symlink(), 'symlink in package path')
    require(binary.is_file() and os.access(binary, os.X_OK), 'missing executable')
    require('R-Term.app/Contents/MacOS/' in relative.as_posix(), 'expected macOS app bundle')
    entries = {entry['path']: entry for entry in manifest['files']}
    require(relative.as_posix() in entries, 'binary absent from manifest')
    require(entries[relative.as_posix()]['sha256'] == digest(binary), 'manifest binary hash mismatch')
    require(manifest['artifact']['runtime_target'] == 'macos-' + {'arm64': 'aarch64', 'x86_64': 'x86_64'}[platform.machine()], 'wrong package architecture')
    return binary, {'source_commit': commit, 'binary': str(binary), 'sha256': digest(binary),
                    'manifest_sha256': digest(manifest_path)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--candidate-ref', required=True)
    parser.add_argument('--candidate-package', type=Path, required=True)
    parser.add_argument('--lkg-package', type=Path, required=True)
    parser.add_argument('--launcher', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    require(platform.system() == 'Darwin', 'macOS native execution required')
    require(re.fullmatch('[0-9a-f]{40}', args.candidate_ref) is not None, 'immutable candidate SHA required')
    output = args.output_dir.absolute()
    require(not output.exists(), 'refusing to overwrite prior evidence')
    output.mkdir(parents=True)
    report = {'schema': 'rterm.stage7/macos-local-sampling/v1', 'status': 'running',
              'scope': 'local performance sampling; not full product or cross-platform certification',
              'runs': [], 'platform': platform.platform(), 'architecture': platform.machine()}
    try:
        contract_path = ROOT / 'scripts/ci/stage7-split-contract.json'
        contract = json.loads(contract_path.read_text())
        report['contract_sha256'] = digest(contract_path)
        report['orchestrator_sha256'] = digest(Path(__file__))
        report['launcher_sha256'] = digest(args.launcher)
        report['identities'] = {}
        binaries = {}
        for role, payload, commit in [('candidate', args.candidate_package, args.candidate_ref),
                                     ('product_lkg', args.lkg_package, contract['product_lkg_ref'])]:
            binaries[role], report['identities'][role] = package_identity(payload.absolute(), commit)
        for name, argv in [('hardware', ['system_profiler', 'SPHardwareDataType', 'SPDisplaysDataType']),
                           ('os', ['sw_vers']), ('power', ['pmset', '-g', 'custom'])]:
            run_record(argv, output / name, 60)
        protocol = contract['sampling']['residence']
        rounds = contract['protocol']
        collected = {scenario: {role: [] for role in binaries} for scenario in ('startup', 'empty-window', 'ssh1')}
        for scenario in collected:
            for index in range(rounds['warmups'] + rounds['measured_cold_processes']):
                phase = 'warmup' if index < rounds['warmups'] else 'measured'
                for role, binary in binaries.items():
                    require(digest(binary) == report['identities'][role]['sha256'], 'binary changed')
                    name = f'{scenario}-{index:02d}-{role}'
                    if scenario == 'startup':
                        argv = [str(binary), 'ssh', '--gui', '--renderer', 'auto', '--host', '127.0.0.1',
                                '--port', '9', '--user', 'rterm-diagnostics', '--password', '--benchmark-startup']
                    else:
                        argv = [str(args.launcher.absolute()), '--app', str(binary), '--scenario', scenario,
                                '--product-gui', '--renderer', 'auto', '--json',
                                '--stabilization-ms', str(protocol['stabilization_ms']),
                                '--sample-interval-ms', str(protocol['sample_interval_ms']),
                                '--sample-count', str(protocol['samples_per_process'])]
                    env = {**os.environ, 'RTERM_BENCHMARK_WINDOW_SCALE_FACTOR': '1'}
                    require(not any(k in env for k in ('RTERM_STAGE7_PRODUCT_GUI_PROBE', 'RTERM_TEST_APP_EXECUTABLE', 'WGPU_BACKEND')), 'inherited diagnostic override')
                    if scenario != 'startup':
                        env['RTERM_DIAGNOSTIC_EVIDENCE_DIR'] = str(output / name / 'raw')
                    stdout, stderr = run_record(argv, output / name, rounds['timeout_seconds'], env)
                    if scenario == 'startup':
                        matches = re.findall(r'^first_present process_to_first_present_ms=(\d+) final_renderer=cpu$', stderr, re.M)
                        require(len(matches) == 1, 'missing or duplicate CPU first-present marker')
                        values = [int(matches[0])]
                    else:
                        data = json.loads(stdout)
                        values = validate_residence(data, scenario, binary, protocol)
                        scan = json.loads((output / name / 'raw/scan.json').read_text())
                        require(scan['complete'] is True, 'raw capture or secret scan incomplete')
                        require(scan['actual_fixture_secret_checked'] is (scenario == 'ssh1'), 'actual fixture secret not checked')
                        adapter = {key: data['renderer'].get(key) for key in
                                   ('adapter_name', 'adapter_vendor_id', 'adapter_device_id', 'adapter_type')}
                        require(bool(adapter['adapter_name']), 'missing adapter identity')
                        require(adapter == report.setdefault('adapter', adapter), 'adapter changed across runs')
                    report['runs'].append({'name': name, 'role': role, 'scenario': scenario, 'phase': phase,
                                           'samples': values, 'files': {p.relative_to(output / name).as_posix(): digest(p)
                                                                      for p in (output / name).rglob('*') if p.is_file()}})
                    if phase == 'measured':
                        collected[scenario][role].append(values)
                    write_json(output / 'report.json', report)
                    print(name, 'ok', flush=True)
        summaries = {scenario: {role: aggregate(runs) for role, runs in roles.items()} for scenario, roles in collected.items()}
        report['summary'] = summaries
        report['comparisons'] = {scenario: comparison(values['candidate'], values['product_lkg'], contract['windows_product_gates']['relative_regression_ratio_max'])
                                 for scenario, values in summaries.items() if scenario != 'startup'}
        for role, binary in binaries.items():
            require(digest(binary) == report['identities'][role]['sha256'], 'binary changed during sampling')
        report['status'] = 'passed' if all(x['passed'] for x in report['comparisons'].values()) else 'failed'
        report['remaining_product_acceptance'] = ['candidate/rollback packaged functional receipts', 'complete secret-scan coverage', 'visible interaction evidence']
    except (OSError, ValueError, KeyError, TypeError) as error:
        report['status'] = 'failed'
        report['error'] = str(error)
    except KeyboardInterrupt:
        report['status'] = 'interrupted'
    finally:
        write_json(output / 'report.json', report)
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
