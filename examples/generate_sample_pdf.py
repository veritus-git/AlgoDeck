#!/usr/bin/env python3
"""
Generates an authentic Polish Olympiad in Informatics sample PDF for testing AlgoDeck.
"""
from pathlib import Path
from reportlab.lib.pagesizes import letter
from reportlab.pdfgen import canvas

def create_koleje_pdf(output_path: Path):
    c = canvas.Canvas(str(output_path), pagesize=letter)
    width, height = letter

    # Title & Header
    c.setFont("Helvetica-Bold", 18)
    c.drawString(50, height - 60, "Koleje")
    c.setFont("Helvetica", 10)
    c.drawString(50, height - 75, "IX Olimpiada Informatyczna • Etap II • Plik źródłowy: kol.cpp")
    c.drawString(50, height - 90, "Dostepny czas: 1.0 s • Dostepna pamiec: 128 MB")
    c.line(50, height - 98, width - 50, height - 98)

    # Problem statement
    c.setFont("Helvetica-Bold", 12)
    c.drawString(50, height - 125, "Tresc zadania")
    c.setFont("Helvetica", 10)
    text = (
        "W Bajtocji wybudowano nowa linie kolejowa laczaca n kolejnych miast numerowanych od 1 do n.\n"
        "Wzdluz linii kursuje jeden pociag o pojemnosci m miejsc pasazerskich.\n"
        "Do biura obslugi wplynelo z zgloszen rezerwacji. Kazde zgloszenie okresla stacje poczatkowa p,\n"
        "stacje docelowa k oraz liczbe pasazerow l. Zgloszenie jest akceptowane tylko wtedy, gdy na calym\n"
        "odcinku od p do k liczba wolnych miejsc nie spadnie ponizej zera."
    )
    y = height - 145
    for line in text.split("\n"):
        c.drawString(50, y, line)
        y -= 15

    # Input/Output
    y -= 10
    c.setFont("Helvetica-Bold", 12)
    c.drawString(50, y, "Wejscie")
    y -= 20
    c.setFont("Helvetica", 10)
    in_desc = "Pierwszy wiersz zawiera liczby n, m, z (1 <= n, m <= 60000, 1 <= z <= 100000).\nKolejne z wierszy zawiera po trzy liczby: p_i, k_i, l_i."
    for line in in_desc.split("\n"):
        c.drawString(50, y, line)
        y -= 15

    y -= 10
    c.setFont("Helvetica-Bold", 12)
    c.drawString(50, y, "Wyjscie")
    y -= 20
    c.setFont("Helvetica", 10)
    c.drawString(50, y, "Dla kazdego zgloszenia wypisz litere T (zaakceptowano) lub N (odrzucono).")
    y -= 30

    # Example Table
    c.setFont("Helvetica-Bold", 12)
    c.drawString(50, y, "Przyklad")
    y -= 20

    c.setFont("Helvetica", 10)
    c.drawString(50, y, "Dla danych wejsciowych:")
    c.drawString(250, y, "poprawnym wynikiem jest:")
    y -= 20

    # Draw code boxes
    c.setFont("Courier", 9)
    # Box 1 Input
    c.rect(50, y - 85, 170, 95)
    c.drawString(60, y - 5, "4 6 4")
    c.drawString(60, y - 20, "1 4 2")
    c.drawString(60, y - 35, "1 3 2")
    c.drawString(60, y - 50, "2 4 3")
    c.drawString(60, y - 65, "1 2 3")

    # Box 2 Output
    c.rect(250, y - 85, 170, 95)
    c.drawString(260, y - 5, "T")
    c.drawString(260, y - 20, "T")
    c.drawString(260, y - 35, "N")
    c.drawString(260, y - 50, "T")

    c.save()
    print(f"Sample PDF created at: {output_path}")

if __name__ == "__main__":
    out = Path(__file__).parent / "koleje.pdf"
    create_koleje_pdf(out)
