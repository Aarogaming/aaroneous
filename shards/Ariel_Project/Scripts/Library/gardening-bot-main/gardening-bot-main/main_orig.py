from core.bot import Bot

if __name__ == "__main__":
    try:
        Bot().run()
    except KeyboardInterrupt:
        print("Execution stopped by user.")
    except Exception as e:
        print(f"An error occurred: {e}")