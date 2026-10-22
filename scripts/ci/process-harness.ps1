Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

if (-not ("RTermCiJobObject" -as [type])) {
  Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public sealed class RTermCiJobObject : IDisposable {
    private const uint JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000;
    private IntPtr handle;

    [StructLayout(LayoutKind.Sequential)]
    private struct IoCounters {
        public ulong ReadOperationCount;
        public ulong WriteOperationCount;
        public ulong OtherOperationCount;
        public ulong ReadTransferCount;
        public ulong WriteTransferCount;
        public ulong OtherTransferCount;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct BasicLimitInformation {
        public long PerProcessUserTimeLimit;
        public long PerJobUserTimeLimit;
        public uint LimitFlags;
        public UIntPtr MinimumWorkingSetSize;
        public UIntPtr MaximumWorkingSetSize;
        public uint ActiveProcessLimit;
        public IntPtr Affinity;
        public uint PriorityClass;
        public uint SchedulingClass;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct ExtendedLimitInformation {
        public BasicLimitInformation BasicLimitInformation;
        public IoCounters IoInfo;
        public UIntPtr ProcessMemoryLimit;
        public UIntPtr JobMemoryLimit;
        public UIntPtr PeakProcessMemoryUsed;
        public UIntPtr PeakJobMemoryUsed;
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateJobObject(IntPtr attributes, string name);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool SetInformationJobObject(
        IntPtr job,
        int informationClass,
        IntPtr information,
        uint informationLength);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CloseHandle(IntPtr handle);

    public RTermCiJobObject() {
        handle = CreateJobObject(IntPtr.Zero, null);
        if (handle == IntPtr.Zero) {
            throw new Win32Exception(Marshal.GetLastWin32Error(), "CreateJobObject failed");
        }
        var limits = new ExtendedLimitInformation();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        int length = Marshal.SizeOf(typeof(ExtendedLimitInformation));
        IntPtr pointer = Marshal.AllocHGlobal(length);
        try {
            Marshal.StructureToPtr(limits, pointer, false);
            if (!SetInformationJobObject(handle, 9, pointer, (uint)length)) {
                throw new Win32Exception(
                    Marshal.GetLastWin32Error(),
                    "SetInformationJobObject(KILL_ON_JOB_CLOSE) failed");
            }
        } catch {
            CloseHandle(handle);
            handle = IntPtr.Zero;
            throw;
        } finally {
            Marshal.FreeHGlobal(pointer);
        }
    }

    public void Assign(Process process) {
        AssignHandle(process.Handle);
    }

    public void AssignHandle(IntPtr processHandle) {
        if (handle == IntPtr.Zero) {
            throw new ObjectDisposedException("RTermCiJobObject");
        }
        if (!AssignProcessToJobObject(handle, processHandle)) {
            throw new Win32Exception(
                Marshal.GetLastWin32Error(),
                "AssignProcessToJobObject failed");
        }
    }

    public void Dispose() {
        if (handle != IntPtr.Zero) {
            CloseHandle(handle);
            handle = IntPtr.Zero;
        }
        GC.SuppressFinalize(this);
    }
}

public sealed class RTermCiOwnedProcess : IDisposable {
    private const uint CREATE_SUSPENDED = 0x00000004;
    private const uint CREATE_UNICODE_ENVIRONMENT = 0x00000400;
    private const uint STARTF_USESTDHANDLES = 0x00000100;
    private const uint HANDLE_FLAG_INHERIT = 0x00000001;
    private const uint GENERIC_READ = 0x80000000;
    private const uint FILE_SHARE_READ = 0x00000001;
    private const uint FILE_SHARE_WRITE = 0x00000002;
    private const uint OPEN_EXISTING = 3;
    private const uint WAIT_OBJECT_0 = 0;
    private const uint WAIT_TIMEOUT = 258;
    private const uint CLEANUP_MILLISECONDS = 10000;

    [StructLayout(LayoutKind.Sequential)]
    private struct SecurityAttributes {
        public uint Length;
        public IntPtr SecurityDescriptor;
        [MarshalAs(UnmanagedType.Bool)]
        public bool InheritHandle;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct StartupInfo {
        public uint Cb;
        public string Reserved;
        public string Desktop;
        public string Title;
        public uint X;
        public uint Y;
        public uint XSize;
        public uint YSize;
        public uint XCountChars;
        public uint YCountChars;
        public uint FillAttribute;
        public uint Flags;
        public short ShowWindow;
        public short ReservedBytes;
        public IntPtr ReservedPointer;
        public IntPtr StandardInput;
        public IntPtr StandardOutput;
        public IntPtr StandardError;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct ProcessInformation {
        public IntPtr Process;
        public IntPtr Thread;
        public uint ProcessId;
        public uint ThreadId;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CreatePipe(
        out IntPtr readPipe,
        out IntPtr writePipe,
        ref SecurityAttributes attributes,
        uint size);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool SetHandleInformation(
        IntPtr handle,
        uint mask,
        uint flags);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateFile(
        string fileName,
        uint desiredAccess,
        uint shareMode,
        ref SecurityAttributes attributes,
        uint creationDisposition,
        uint flagsAndAttributes,
        IntPtr templateFile);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool CreateProcessW(
        string applicationName,
        StringBuilder commandLine,
        IntPtr processAttributes,
        IntPtr threadAttributes,
        [MarshalAs(UnmanagedType.Bool)] bool inheritHandles,
        uint creationFlags,
        IntPtr environment,
        string currentDirectory,
        ref StartupInfo startupInfo,
        out ProcessInformation processInformation);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint ResumeThread(IntPtr thread);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool TerminateProcess(IntPtr process, uint exitCode);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetExitCodeProcess(IntPtr process, out uint exitCode);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CloseHandle(IntPtr handle);

    private RTermCiJobObject job;
    private IntPtr nativeProcessHandle;
    public Process Process { get; private set; }
    public StreamReader StandardOutput { get; private set; }
    public StreamReader StandardError { get; private set; }
    // Stopwatch timestamp captured immediately before the suspended child is
    // resumed. Startup probes use the conservative pre-resume boundary so the
    // child cannot execute before their external measurement begins.
    public long ResumeTimestamp { get; private set; }

    private RTermCiOwnedProcess(
        RTermCiJobObject job,
        IntPtr nativeProcessHandle,
        Process process,
        StreamReader standardOutput,
        StreamReader standardError,
        long resumeTimestamp) {
        this.job = job;
        this.nativeProcessHandle = nativeProcessHandle;
        Process = process;
        StandardOutput = standardOutput;
        StandardError = standardError;
        ResumeTimestamp = resumeTimestamp;
    }

    public static RTermCiOwnedProcess Start(
        string filePath,
        string commandLine,
        string workingDirectory,
        bool failAssignmentForTest) {
        var attributes = new SecurityAttributes {
            Length = (uint)Marshal.SizeOf(typeof(SecurityAttributes)),
            InheritHandle = true,
        };
        IntPtr stdoutRead = IntPtr.Zero;
        IntPtr stdoutWrite = IntPtr.Zero;
        IntPtr stderrRead = IntPtr.Zero;
        IntPtr stderrWrite = IntPtr.Zero;
        IntPtr standardInput = IntPtr.Zero;
        var processInformation = new ProcessInformation();
        RTermCiJobObject job = null;
        Process process = null;
        StreamReader stdoutReader = null;
        StreamReader stderrReader = null;
        bool created = false;
        try {
            CreateParentReadPipe(ref attributes, out stdoutRead, out stdoutWrite, "stdout");
            CreateParentReadPipe(ref attributes, out stderrRead, out stderrWrite, "stderr");
            standardInput = CreateFile(
                "NUL",
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ref attributes,
                OPEN_EXISTING,
                0,
                IntPtr.Zero);
            if (standardInput == new IntPtr(-1)) {
                throw LastError("CreateFile(NUL) failed");
            }
            var startup = new StartupInfo {
                Cb = (uint)Marshal.SizeOf(typeof(StartupInfo)),
                Flags = STARTF_USESTDHANDLES,
                StandardInput = standardInput,
                StandardOutput = stdoutWrite,
                StandardError = stderrWrite,
            };
            job = new RTermCiJobObject();
            if (!CreateProcessW(
                    filePath,
                    new StringBuilder(commandLine),
                    IntPtr.Zero,
                    IntPtr.Zero,
                    true,
                    CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
                    IntPtr.Zero,
                    workingDirectory,
                    ref startup,
                    out processInformation)) {
                throw LastError("CreateProcessW(CREATE_SUSPENDED) failed");
            }
            created = true;
            Close(ref stdoutWrite);
            Close(ref stderrWrite);
            Close(ref standardInput);
            stdoutReader = CreateReader(ref stdoutRead);
            stderrReader = CreateReader(ref stderrRead);
            process = Process.GetProcessById((int)processInformation.ProcessId);
            if (failAssignmentForTest) {
                throw new InvalidOperationException("simulated AssignProcessToJobObject failure");
            }
            job.AssignHandle(processInformation.Process);
            long resumeTimestamp = Stopwatch.GetTimestamp();
            if (ResumeThread(processInformation.Thread) == UInt32.MaxValue) {
                throw LastError("ResumeThread failed");
            }
            Close(ref processInformation.Thread);
            IntPtr ownedNativeProcess = processInformation.Process;
            processInformation.Process = IntPtr.Zero;
            return new RTermCiOwnedProcess(
                job,
                ownedNativeProcess,
                process,
                stdoutReader,
                stderrReader,
                resumeTimestamp);
        } catch {
            Exception cleanupFailure = null;
            if (created) {
                TerminateProcess(processInformation.Process, 1);
            }
            if (job != null) {
                job.Dispose();
            }
            if (created) {
                uint wait = WaitForSingleObject(processInformation.Process, CLEANUP_MILLISECONDS);
                if (wait == WAIT_TIMEOUT) {
                    cleanupFailure = new TimeoutException(
                        "assignment-failure cleanup exceeded its 10s deadline");
                } else if (wait != WAIT_OBJECT_0) {
                    cleanupFailure = LastError("assignment-failure cleanup wait failed");
                }
            }
            if (stdoutReader != null) stdoutReader.Dispose();
            if (stderrReader != null) stderrReader.Dispose();
            if (process != null) process.Dispose();
            Close(ref stdoutRead);
            Close(ref stdoutWrite);
            Close(ref stderrRead);
            Close(ref stderrWrite);
            Close(ref standardInput);
            Close(ref processInformation.Thread);
            Close(ref processInformation.Process);
            if (cleanupFailure != null) {
                throw cleanupFailure;
            }
            throw;
        }
    }

    public void CloseJob() {
        if (job != null) {
            job.Dispose();
            job = null;
        }
    }

    public static Process CaptureProcessIdentity(int processId, long expectedStartTimeUtcTicks) {
        Process process = null;
        try {
            process = Process.GetProcessById(processId);
            IntPtr unused = process.Handle;
            long actualStartTimeUtcTicks = process.StartTime.ToUniversalTime().Ticks;
            if (actualStartTimeUtcTicks != expectedStartTimeUtcTicks) {
                throw new InvalidOperationException(
                    "process identity mismatch for PID " + processId +
                    ": observed start ticks " + actualStartTimeUtcTicks +
                    ", expected " + expectedStartTimeUtcTicks);
            }
            return process;
        } catch {
            if (process != null) process.Dispose();
            throw;
        }
    }

    public int ExitCode {
        get {
            uint exitCode;
            if (!GetExitCodeProcess(nativeProcessHandle, out exitCode)) {
                throw LastError("GetExitCodeProcess failed");
            }
            return unchecked((int)exitCode);
        }
    }

    public void Dispose() {
        CloseJob();
        if (StandardOutput != null) StandardOutput.Dispose();
        if (StandardError != null) StandardError.Dispose();
        if (Process != null) Process.Dispose();
        Close(ref nativeProcessHandle);
        GC.SuppressFinalize(this);
    }

    private static void CreateParentReadPipe(
        ref SecurityAttributes attributes,
        out IntPtr readPipe,
        out IntPtr writePipe,
        string name) {
        if (!CreatePipe(out readPipe, out writePipe, ref attributes, 0)) {
            throw LastError("CreatePipe(" + name + ") failed");
        }
        if (!SetHandleInformation(readPipe, HANDLE_FLAG_INHERIT, 0)) {
            throw LastError("SetHandleInformation(" + name + ") failed");
        }
    }

    private static StreamReader CreateReader(ref IntPtr handle) {
        var safeHandle = new SafeFileHandle(handle, true);
        handle = IntPtr.Zero;
        var stream = new FileStream(safeHandle, FileAccess.Read, 4096, false);
        return new StreamReader(stream, new UTF8Encoding(false), true);
    }

    private static Win32Exception LastError(string message) {
        return new Win32Exception(Marshal.GetLastWin32Error(), message);
    }

    private static void Close(ref IntPtr handle) {
        if (handle != IntPtr.Zero && handle != new IntPtr(-1)) {
            CloseHandle(handle);
        }
        handle = IntPtr.Zero;
    }
}
'@
}

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path

function ConvertTo-WindowsCommandLineArgument {
  param(
    [Parameter(Mandatory = $true)]
    [AllowEmptyString()]
    [string] $Argument
  )

  if ($Argument -match '^[^\s"]+$') {
    return $Argument
  }

  $quoted = [Text.StringBuilder]::new()
  $null = $quoted.Append('"')
  $backslashes = 0
  foreach ($character in $Argument.ToCharArray()) {
    if ($character -eq [char]92) {
      $backslashes++
      continue
    }
    if ($character -eq [char]34) {
      $null = $quoted.Append(([char]92).ToString() * (($backslashes * 2) + 1))
      $null = $quoted.Append('"')
      $backslashes = 0
      continue
    }
    $null = $quoted.Append(([char]92).ToString() * $backslashes)
    $backslashes = 0
    $null = $quoted.Append($character)
  }
  $null = $quoted.Append(([char]92).ToString() * ($backslashes * 2))
  $null = $quoted.Append('"')
  return $quoted.ToString()
}

function Assert-WindowsCommandLineQuoting {
  $cases = @(
    @("", '""'),
    @("plain", "plain"),
    @("two words", '"two words"'),
    @('quote"inside', '"quote\"inside"'),
    @('C:\path with space\', '"C:\path with space\\"')
  )
  foreach ($case in $cases) {
    $observed = ConvertTo-WindowsCommandLineArgument -Argument $case[0]
    if ($observed -cne $case[1]) {
      throw "Windows command-line quoting self-check failed for '$($case[0])': observed '$observed', expected '$($case[1])'"
    }
  }
}

function Get-RemainingMilliseconds {
  param([Parameter(Mandatory = $true)] [DateTimeOffset] $Deadline)

  $remaining = [Math]::Ceiling(($Deadline - [DateTimeOffset]::UtcNow).TotalMilliseconds)
  if ($remaining -le 0) {
    return 0
  }
  return [int] [Math]::Min($remaining, [int]::MaxValue)
}

function Complete-StreamBeforeDeadline {
  param(
    [Parameter(Mandatory = $true)] $Task,
    [Parameter(Mandatory = $true)] [DateTimeOffset] $Deadline,
    [Parameter(Mandatory = $true)] [string] $StreamName
  )

  $remaining = Get-RemainingMilliseconds -Deadline $Deadline
  if ($remaining -le 0 -or -not $Task.Wait($remaining)) {
    throw "$StreamName did not complete before the shared phase deadline"
  }
  return $Task.Result
}

function Complete-ProcessIdentityBeforeDeadline {
  param(
    [Parameter(Mandatory = $true)]
    [Diagnostics.Process] $Process,

    [Parameter(Mandatory = $true)]
    [DateTimeOffset] $Deadline,

    [Parameter(Mandatory = $true)]
    [string] $Name
  )

  if ($Process.HasExited) {
    return
  }
  $remaining = Get-RemainingMilliseconds -Deadline $Deadline
  if ($remaining -le 0 -or -not $Process.WaitForExit($remaining)) {
    throw "$Name did not exit before the shared cleanup deadline"
  }
}

function Resolve-NativeExecutable {
  param([Parameter(Mandatory = $true)] [string] $FilePath)

  if ([IO.Path]::IsPathRooted($FilePath)) {
    return (Resolve-Path -LiteralPath $FilePath).Path
  }
  $command = Get-Command $FilePath -CommandType Application -ErrorAction Stop |
    Select-Object -First 1
  return $command.Source
}

function Invoke-BoundedProcess {
  param(
    [Parameter(Mandatory = $true)]
    [string] $Phase,

    [Parameter(Mandatory = $true)]
    [string] $FilePath,

    [string[]] $ArgumentList = @(),

    [Parameter(Mandatory = $true)]
    [int] $TimeoutSeconds,

    [scriptblock] $CaptureDescendantForTest,

    [scriptblock] $ObserveStartedProcessForTest,

    [switch] $FailAssignmentForTest
  )

  $resolvedFilePath = Resolve-NativeExecutable -FilePath $FilePath
  $commandLine = @(
    ConvertTo-WindowsCommandLineArgument -Argument $resolvedFilePath
    $ArgumentList | ForEach-Object {
      ConvertTo-WindowsCommandLineArgument -Argument $_
    }
  ) -join " "

  $ownedProcess = $null
  $process = $null
  $started = $false
  $jobClosed = $false
  $cleanupDeadline = $null
  $capturedDescendant = $null
  $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSeconds)
  try {
    $ownedProcess = [RTermCiOwnedProcess]::Start(
      $resolvedFilePath,
      $commandLine,
      $repositoryRoot,
      $FailAssignmentForTest.IsPresent
    )
    $process = $ownedProcess.Process
    $started = $true
    $stdout = $ownedProcess.StandardOutput.ReadToEndAsync()
    $stderr = $ownedProcess.StandardError.ReadToEndAsync()
    if ($null -ne $ObserveStartedProcessForTest) {
      $null = & $ObserveStartedProcessForTest $process
    }
    $remaining = Get-RemainingMilliseconds -Deadline $deadline
    if ($remaining -le 0 -or -not $process.WaitForExit($remaining)) {
      $cleanupDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
      if ($null -ne $CaptureDescendantForTest) {
        $capturedDescendant = & $CaptureDescendantForTest $cleanupDeadline
      }
      $ownedProcess.CloseJob()
      $jobClosed = $true
      $cleanupRemaining = Get-RemainingMilliseconds -Deadline $cleanupDeadline
      if ($cleanupRemaining -le 0 -or -not $process.WaitForExit($cleanupRemaining)) {
        throw "$Phase exceeded its ${TimeoutSeconds}s deadline and process-tree cleanup did not finish"
      }
      if ($null -ne $capturedDescendant) {
        Complete-ProcessIdentityBeforeDeadline `
          -Process $capturedDescendant `
          -Deadline $cleanupDeadline `
          -Name "$Phase captured descendant"
      }
      $timeoutStdout = Complete-StreamBeforeDeadline -Task $stdout -Deadline $cleanupDeadline -StreamName "$Phase stdout"
      $timeoutStderr = Complete-StreamBeforeDeadline -Task $stderr -Deadline $cleanupDeadline -StreamName "$Phase stderr"
      throw "$Phase exceeded its ${TimeoutSeconds}s deadline; the process tree was killed and reaped`nstdout:`n$timeoutStdout`nstderr:`n$timeoutStderr"
    }
    $stdoutText = Complete-StreamBeforeDeadline -Task $stdout -Deadline $deadline -StreamName "$Phase stdout"
    $stderrText = Complete-StreamBeforeDeadline -Task $stderr -Deadline $deadline -StreamName "$Phase stderr"
    $exitCode = $ownedProcess.ExitCode
    if ($exitCode -ne 0) {
      throw "$Phase failed with exit code $exitCode`nstdout:`n$stdoutText`nstderr:`n$stderrText"
    }
    if (-not [string]::IsNullOrEmpty($stdoutText)) {
      Write-Host $stdoutText.TrimEnd()
    }
    if (-not [string]::IsNullOrEmpty($stderrText)) {
      Write-Host $stderrText.TrimEnd()
    }
    return [pscustomobject]@{
      Stdout = $stdoutText
      Stderr = $stderrText
    }
  } finally {
    if ($null -ne $ownedProcess -and -not $jobClosed) {
      $ownedProcess.CloseJob()
      $jobClosed = $true
    }
    if ($started -and -not $process.HasExited) {
      if ($null -eq $cleanupDeadline) {
        $cleanupDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
      }
      $cleanupRemaining = Get-RemainingMilliseconds -Deadline $cleanupDeadline
      if ($cleanupRemaining -le 0 -or -not $process.WaitForExit($cleanupRemaining)) {
        throw "$Phase process-tree cleanup did not finish within 10s"
      }
    }
    if ($null -ne $ownedProcess) {
      $ownedProcess.Dispose()
    }
    if ($null -ne $capturedDescendant) {
      $capturedDescendant.Dispose()
    }
  }
}

function Assert-BoundedProcessHarness {
  Assert-WindowsCommandLineQuoting

  $roundTripArguments = @("", "two words", 'quote"inside', 'C:\path with space\')
  $echoScriptPath = Join-Path ([IO.Path]::GetTempPath()) "rterm-ci-argv-$PID-$([Guid]::NewGuid().ToString('N')).ps1"
  [IO.File]::WriteAllText(
    $echoScriptPath,
    '[Console]::OutputEncoding=[Text.UTF8Encoding]::new(); [Console]::Write(($args | ConvertTo-Json -Compress))',
    [Text.UTF8Encoding]::new($false)
  )
  try {
    $roundTripParameters = @{
      Phase = "quoted argv round-trip"
      FilePath = "powershell.exe"
      ArgumentList = @("-NoProfile", "-NonInteractive", "-File", $echoScriptPath) + $roundTripArguments
      # Fresh hosted Windows images can delay the first PowerShell child while
      # Defender and the profile cache initialize. This remains a bounded
      # startup self-test; it only avoids treating that cold-start jitter as a
      # command-line quoting failure.
      TimeoutSeconds = 30
    }
    $roundTrip = Invoke-BoundedProcess @roundTripParameters
  } finally {
    if (Test-Path -LiteralPath $echoScriptPath) {
      Remove-Item -LiteralPath $echoScriptPath -Force
    }
  }
  $expectedArgumentsJson = ConvertTo-Json -InputObject $roundTripArguments -Compress
  if ($roundTrip.Stdout -cne $expectedArgumentsJson) {
    throw "quoted argv round-trip mismatch: observed '$($roundTrip.Stdout)', expected '$expectedArgumentsJson'"
  }

  $assignmentSentinel = Join-Path ([IO.Path]::GetTempPath()) "rterm-ci-assign-$PID-$([Guid]::NewGuid().ToString('N')).txt"
  $previousAssignmentSentinel = $env:RTERM_CI_ASSIGN_SENTINEL
  $env:RTERM_CI_ASSIGN_SENTINEL = $assignmentSentinel
  try {
    $assignmentFailed = $false
    try {
      $assignmentParameters = @{
        Phase = "suspended assignment failure self-test"
        FilePath = "powershell.exe"
        ArgumentList = @(
          "-NoProfile",
          "-NonInteractive",
          "-Command",
          '[IO.File]::WriteAllText($env:RTERM_CI_ASSIGN_SENTINEL, "escaped")'
        )
        TimeoutSeconds = 10
        FailAssignmentForTest = $true
      }
      $null = Invoke-BoundedProcess @assignmentParameters
    } catch {
      $assignmentFailed = $_.Exception.Message -match 'simulated AssignProcessToJobObject failure'
    }
    if (-not $assignmentFailed) {
      throw "suspended assignment failure self-test did not report the simulated failure"
    }
    if (Test-Path -LiteralPath $assignmentSentinel) {
      throw "suspended assignment failure self-test allowed the unassigned child to execute"
    }
  } finally {
    $env:RTERM_CI_ASSIGN_SENTINEL = $previousAssignmentSentinel
    if (Test-Path -LiteralPath $assignmentSentinel) {
      Remove-Item -LiteralPath $assignmentSentinel -Force
    }
  }

  $identityMismatchRejected = $false
  $unexpectedIdentity = $null
  try {
    $unexpectedIdentity = [RTermCiOwnedProcess]::CaptureProcessIdentity($PID, 0)
  } catch {
    $identityMismatchRejected = $_.Exception.Message -match "process identity mismatch"
  } finally {
    if ($null -ne $unexpectedIdentity) {
      $unexpectedIdentity.Dispose()
    }
  }
  if (-not $identityMismatchRejected) {
    throw "process identity capture accepted a reused PID identity"
  }

  $runningProcess = Start-Process powershell.exe -WindowStyle Hidden -ArgumentList @(
    "-NoProfile",
    "-NonInteractive",
    "-Command",
    '$wait=[Threading.ManualResetEvent]::new($false);$wait.WaitOne()'
  ) -PassThru
  $runningIdentity = $null
  try {
    $runningIdentity = [RTermCiOwnedProcess]::CaptureProcessIdentity(
      $runningProcess.Id,
      $runningProcess.StartTime.ToUniversalTime().Ticks
    )
    $runningProcessRejected = $false
    try {
      Complete-ProcessIdentityBeforeDeadline `
        -Process $runningIdentity `
        -Deadline ([DateTimeOffset]::UtcNow.AddMilliseconds(100)) `
        -Name "running process identity self-test"
    } catch {
      $runningProcessRejected = $_.Exception.Message -match "did not exit before the shared cleanup deadline"
    }
    if (-not $runningProcessRejected) {
      throw "process identity cleanup accepted a process that was still running"
    }
  } finally {
    if (-not $runningProcess.HasExited) {
      Stop-Process -Id $runningProcess.Id -Force
      if (-not $runningProcess.WaitForExit(10000)) {
        throw "running process identity self-test cleanup exceeded 10s"
      }
    }
    if ($null -ne $runningIdentity) {
      $runningIdentity.Dispose()
    }
    $runningProcess.Dispose()
  }

  $identity = [Guid]::NewGuid().ToString("N")
  $sentinel = Join-Path ([IO.Path]::GetTempPath()) "rterm-ci-job-$PID-$identity.json"
  $readyEventName = "Local\rterm-ci-job-ready-$PID-$identity"
  $readyEvent = [Threading.EventWaitHandle]::new(
    $false,
    [Threading.EventResetMode]::ManualReset,
    $readyEventName
  )
  $previousSentinel = $env:RTERM_CI_JOB_SENTINEL
  $previousReadyEvent = $env:RTERM_CI_JOB_READY_EVENT
  $env:RTERM_CI_JOB_SENTINEL = $sentinel
  $env:RTERM_CI_JOB_READY_EVENT = $readyEventName
  $timeoutScript = @'
$grandchild = Start-Process powershell.exe -WindowStyle Hidden -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', '$wait=[Threading.ManualResetEvent]::new($false);$wait.WaitOne()') -PassThru
$grandchildIdentity = [ordered]@{
  pid = $grandchild.Id
  start_time_utc_ticks = $grandchild.StartTime.ToUniversalTime().Ticks
}
[IO.File]::WriteAllText(
  $env:RTERM_CI_JOB_SENTINEL,
  ($grandchildIdentity | ConvertTo-Json -Compress)
)
# Publishing readiness allows the parent to terminate this process immediately.
# Both streams must already contain the markers before that handoff.
[Console]::Out.Write('timeout-stdout-marker')
[Console]::Error.Write('timeout-stderr-marker')
[Console]::Out.Flush()
[Console]::Error.Flush()
$ready = [Threading.EventWaitHandle]::OpenExisting($env:RTERM_CI_JOB_READY_EVENT)
try {
  $null = $ready.Set()
} finally {
  $ready.Dispose()
}
$wait = [Threading.ManualResetEvent]::new($false)
$null = $wait.WaitOne()
'@
  try {
    $timedOut = $false
    $timeoutDiagnostics = ""
    try {
      $captureDescendant = {
        param([Parameter(Mandatory = $true)] [DateTimeOffset] $CleanupDeadline)

        $remaining = [Math]::Ceiling(
          ($CleanupDeadline - [DateTimeOffset]::UtcNow).TotalMilliseconds
        )
        $remaining = [int] [Math]::Max(
          0,
          [Math]::Min($remaining, [int]::MaxValue)
        )
        if ($remaining -le 0 -or -not $readyEvent.WaitOne($remaining)) {
          throw "job object timeout self-test did not observe its grandchild before cleanup deadline"
        }
        $grandchildIdentity = Get-Content -LiteralPath $sentinel -Raw | ConvertFrom-Json
        return [RTermCiOwnedProcess]::CaptureProcessIdentity(
          [int] $grandchildIdentity.pid,
          [long] $grandchildIdentity.start_time_utc_ticks
        )
      }.GetNewClosure()
      $timeoutParameters = @{
        Phase = "job object timeout self-test"
        FilePath = "powershell.exe"
        ArgumentList = @("-NoProfile", "-NonInteractive", "-Command", $timeoutScript)
        TimeoutSeconds = 2
        CaptureDescendantForTest = $captureDescendant
      }
      $null = Invoke-BoundedProcess @timeoutParameters
    } catch {
      $timedOut = $true
      $timeoutDiagnostics = $_.Exception.Message
    }
    if (-not $timedOut -or $timeoutDiagnostics -notmatch 'exceeded its 2s deadline') {
      throw "job object timeout self-test did not report its deadline: $timeoutDiagnostics"
    }
    foreach ($marker in @("timeout-stdout-marker", "timeout-stderr-marker")) {
      if (-not $timeoutDiagnostics.Contains($marker)) {
        throw "job object timeout self-test did not drain $marker"
      }
    }
    if (-not (Test-Path -LiteralPath $sentinel)) {
      throw "job object timeout self-test did not record its grandchild PID"
    }
  } finally {
    $env:RTERM_CI_JOB_SENTINEL = $previousSentinel
    $env:RTERM_CI_JOB_READY_EVENT = $previousReadyEvent
    $readyEvent.Dispose()
    if (Test-Path -LiteralPath $sentinel) {
      Remove-Item -LiteralPath $sentinel -Force
    }
  }
}
