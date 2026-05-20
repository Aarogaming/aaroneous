class Logger:
    def __init__(self):
        self.last_message = None

    def log(self, message):
        """Log a message only if it's different from the last one."""
        if message != self.last_message:
            print(message)  # Print to console (you can also log to a file here)
            self.last_message = message
