import psutil
import os
import signal

def kill_zombies():
    killed = 0
    for proc in psutil.process_iter(['pid', 'name', 'cmdline']):
        try:
            cmdline = proc.info.get('cmdline') or []
            name = proc.info.get('name', '').lower()
            
            # Skip node/AgentGateway
            if 'node' in name or 'agentgateway' in name.lower():
                continue
                
            cmd_str = ' '.join(cmdline).lower()
            
            # Look for our specific scripts
            is_orchestrator = 'run_orchestrator.py' in cmd_str or 'etl_orchestrator.py' in cmd_str
            is_uvx_python = ('python' in name or 'uv' in name) and is_orchestrator
            
            if is_uvx_python:
                print(f"Killing zombie: PID {proc.info['pid']} - {cmd_str}")
                os.kill(proc.info['pid'], signal.SIGTERM)
                killed += 1
        except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess):
            pass
            
    print(f"Total zombies killed: {killed}")

if __name__ == '__main__':
    kill_zombies()
