# Master Syllabus-Based Engineering Formulas
# Extracted from the provided handwritten image

base_formulas = [
    r"R = \frac{\rho \cdot L}{A}",
    # Series
    r"R_T = R_1 + R_2 + \dots",
    r"I_T = I_1 = I_2 = \dots",
    r"V_T = V_1 + V_2 + \dots",
    r"P_T = P_1 + P_2 + \dots",
    # Parallel
    r"R_T = \frac{1}{\frac{1}{R_1} + \frac{1}{R_2} + \dots}",
    r"I_T = I_1 + I_2 + \dots",
    r"V_T = V_1 = V_2 = \dots",
    # Capacitors
    r"C_{T(series)} = \frac{1}{\frac{1}{C_1} + \frac{1}{C_2} + \dots}",
    r"C_{T(parallel)} = C_1 + C_2 + \dots",
    # Inductors
    r"L_{T(series)} = L_1 + L_2 + \dots",
    r"L_{T(parallel)} = \frac{1}{\frac{1}{L_1} + \frac{1}{L_2} + \dots}",
    # Time Constants
    r"\tau = R \cdot C",
    r"\tau = \frac{L}{R}",
    # Reactance
    r"X_C = \frac{1}{2 \pi f C}",
    r"X_L = 2 \pi f L",
    # AC Voltage
    r"V_{RMS} = V_{peak} \cdot 0.707",
    r"V_{peak} = V_{RMS} \cdot 1.414",
    # Impedance
    r"Z = \sqrt{R^2 + X^2}",
    r"Z = \sqrt{R^2 + (X_L - X_C)^2}",
    # Power
    r"S = V \cdot I \quad \text{(Apparent Power)}",
    r"P = V \cdot I \cdot \cos(\theta) \quad \text{(True Power)}",
    r"Q = V \cdot I \cdot \sin(\theta) \quad \text{(Reactive Power)}",
    r"PF = \cos(\theta) = \frac{P}{S}",
]

wye_delta_formulas = [
    # 3-Phase Wye (Y)
    r"V_{Line} = V_{Phase} \cdot \sqrt{3}",
    r"I_{Line} = I_{Phase}",
    # 3-Phase Delta (\Delta)
    r"V_{Line} = V_{Phase}",
    r"I_{Line} = I_{Phase} \cdot \sqrt{3}",
    # 3-Phase Power
    r"P_T = \sqrt{3} \cdot V_{Line} \cdot I_{Line} \cdot PF",
]

# ETE 120 must not include Wye (Y) or Delta formulas.
ETE_120_FORMULAS = base_formulas.copy()

# ETM 110 must include Wye and Delta formulas.
ETM_110_FORMULAS = base_formulas + wye_delta_formulas
