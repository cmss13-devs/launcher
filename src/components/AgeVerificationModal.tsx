import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Modal, ModalContent } from "./Modal";

interface AgeVerificationModalProps {
  visible: boolean;
  onVerified: () => void;
  onClose: () => void;
}

type DatePart = "year" | "month" | "day";

function getLocaleDateOrder(): DatePart[] {
  const formatter = new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
  const parts = formatter.formatToParts(new Date(2000, 0, 2));
  const order: DatePart[] = [];
  for (const part of parts) {
    if (part.type === "year") order.push("year");
    else if (part.type === "month") order.push("month");
    else if (part.type === "day") order.push("day");
  }
  return order.length === 3 ? order : ["month", "day", "year"];
}

const FIELD_CONFIG: Record<DatePart, { placeholder: string; maxLength: number }> = {
  year: { placeholder: "YYYY", maxLength: 4 },
  month: { placeholder: "MM", maxLength: 2 },
  day: { placeholder: "DD", maxLength: 2 },
};

export const AgeVerificationModal = ({
  visible,
  onVerified,
  onClose,
}: AgeVerificationModalProps) => {
  const { t } = useTranslation();
  const [year, setYear] = useState("");
  const [month, setMonth] = useState("");
  const [day, setDay] = useState("");
  const [error, setError] = useState<string | null>(null);

  const dateOrder = useMemo(() => getLocaleDateOrder(), []);

  const setters: Record<DatePart, (v: string) => void> = {
    year: setYear,
    month: setMonth,
    day: setDay,
  };
  const values: Record<DatePart, string> = { year, month, day };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    const y = Number.parseInt(year, 10);
    const m = Number.parseInt(month, 10);
    const d = Number.parseInt(day, 10);

    if (Number.isNaN(y) || Number.isNaN(m) || Number.isNaN(d)) {
      setError(t("age.invalidDate"));
      return;
    }

    const dob = new Date(y, m - 1, d);
    if (
      dob.getFullYear() !== y ||
      dob.getMonth() !== m - 1 ||
      dob.getDate() !== d
    ) {
      setError(t("age.invalidDate"));
      return;
    }

    const now = new Date();
    let age = now.getFullYear() - dob.getFullYear();
    const monthDiff = now.getMonth() - dob.getMonth();
    if (monthDiff < 0 || (monthDiff === 0 && now.getDate() < dob.getDate())) {
      age--;
    }

    if (age < 18) {
      setError(t("age.tooYoung"));
      return;
    }

    onVerified();
  };

  return (
    <Modal visible={visible} onClose={onClose} closeOnOverlayClick title={t("age.title")}>
      <ModalContent>
        <p>{t("age.prompt")}</p>
        <form onSubmit={handleSubmit} className="age-verification-form">
          <div className="dob-inputs">
            {dateOrder.map((part, i) => {
              const config = FIELD_CONFIG[part];
              return (
                <input
                  key={part}
                  type="text"
                  placeholder={config.placeholder}
                  value={values[part]}
                  onChange={(e) => setters[part](e.target.value)}
                  maxLength={config.maxLength}
                  inputMode="numeric"
                  autoFocus={i === 0}
                />
              );
            })}
          </div>
          <p className="age-disclaimer">{t("age.disclaimer")}</p>
          {error && <p className="auth-error-message">{error}</p>}
          <button
            type="submit"
            className="button"
            disabled={!year || !month || !day}
          >
            {t("common.confirm")}
          </button>
        </form>
      </ModalContent>
    </Modal>
  );
};
